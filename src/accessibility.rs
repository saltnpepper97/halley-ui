#![doc = include_str!("../docs/accessibility.md")]
//! AccessKit tree and Linux AT-SPI bridge. Hosts retain input and window ownership.
use crate::ui::{PreparedView, SemanticRole};
use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Role, TextPosition, TextSelection, TreeId,
    TreeInfo, TreeUpdate,
};
use std::collections::{HashMap, HashSet};
use unicode_segmentation::UnicodeSegmentation;

pub use accesskit;
#[derive(Clone, Debug)]
pub enum AccessibleAction {
    Focus(String),
    Activate(String),
    SetValue {
        key: String,
        value: String,
    },
    SetSelection {
        key: String,
        anchor: usize,
        cursor: usize,
    },
    Scroll {
        key: String,
        x: f32,
        y: f32,
    },
    Reveal(String),
}
/// Retain one tree per native window/view. Widget keys keep stable IDs between updates.
#[derive(Default)]
pub struct AccessibilityTree {
    ids: HashMap<(String, u8), NodeId>,
    targets: HashMap<NodeId, (String, u8)>,
    values: HashMap<String, String>,
    next_id: u64,
}
impl AccessibilityTree {
    fn id(&mut self, key: &str, kind: u8) -> NodeId {
        let tuple = (key.into(), kind);
        *self.ids.entry(tuple).or_insert_with(|| {
            self.next_id += 1;
            NodeId(self.next_id + 1)
        })
    }
    pub fn update(&mut self, prepared: &PreparedView, title: &str) -> TreeUpdate {
        let root = NodeId(1);
        let mut nodes = Vec::new();
        let mut live = HashSet::new();
        self.targets.clear();
        self.values.clear();
        for semantic in &prepared.semantics {
            let id = self.id(&semantic.key, 0);
            live.insert((semantic.key.clone(), 0));
            self.targets.insert(id, (semantic.key.clone(), 0));
            let role = match semantic.role {
                SemanticRole::Group | SemanticRole::Scroll => Role::GenericContainer,
                SemanticRole::Label => Role::Label,
                SemanticRole::Button => Role::Button,
                SemanticRole::TextInput => Role::TextInput,
                SemanticRole::Image => Role::Image,
            };
            let mut node = Node::new(role);
            node.set_bounds(bounds(semantic.rect));
            node.set_clips_children();
            node.set_children(
                semantic
                    .children
                    .iter()
                    .map(|key| self.id(key, 0))
                    .collect::<Vec<_>>(),
            );
            if let Some(label) = &semantic.label {
                node.set_label(label.clone());
            }
            if semantic.disabled {
                node.set_disabled();
            }
            if semantic.read_only {
                node.set_read_only();
            }
            match semantic.role {
                SemanticRole::Button => {
                    if semantic.label.is_none()
                        && let Some(value) = &semantic.value
                    {
                        node.set_label(value.clone());
                    }
                    if !semantic.disabled {
                        node.add_action(Action::Focus);
                        node.add_action(Action::Click);
                        node.add_action(Action::ScrollIntoView);
                    }
                }
                SemanticRole::Label => {
                    if let Some(value) = &semantic.value {
                        node.set_value(value.clone());
                    }
                }
                SemanticRole::Image if semantic.label.is_none() => node.set_hidden(),
                SemanticRole::TextInput => {
                    let value = semantic.value.as_deref().unwrap_or("");
                    self.values.insert(semantic.key.clone(), value.into());
                    node.set_value(value);
                    let run_id = self.id(&semantic.key, 1);
                    live.insert((semantic.key.clone(), 1));
                    self.targets.insert(run_id, (semantic.key.clone(), 1));
                    let mut run = Node::new(Role::TextRun);
                    run.set_value(value);
                    // Match selectable graphemes; exceptionally long clusters use scalar chunks.
                    run.set_character_lengths(
                        units(value)
                            .iter()
                            .map(|value| value.len() as u8)
                            .collect::<Vec<_>>(),
                    );
                    run.set_bounds(bounds(semantic.rect));
                    node.set_children(vec![run_id]);
                    if let Some((anchor, cursor)) = semantic.selection {
                        node.set_text_selection(TextSelection {
                            anchor: TextPosition {
                                node: run_id,
                                character_index: char_index(value, anchor),
                            },
                            focus: TextPosition {
                                node: run_id,
                                character_index: char_index(value, cursor),
                            },
                        });
                    }
                    if !semantic.disabled {
                        node.add_action(Action::Focus);
                        node.add_action(Action::SetTextSelection);
                        node.add_action(Action::ScrollIntoView);
                        if !semantic.read_only {
                            node.add_action(Action::SetValue);
                        }
                    }
                    nodes.push((run_id, run));
                }
                SemanticRole::Scroll => {
                    if let Some(scroll) = prepared
                        .scrolls
                        .iter()
                        .find(|scroll| scroll.key == semantic.key)
                    {
                        node.set_scroll_x(scroll.offset.x as f64);
                        node.set_scroll_y(scroll.offset.y as f64);
                        node.set_scroll_x_min(0.0);
                        node.set_scroll_y_min(0.0);
                        node.set_scroll_x_max(
                            (scroll.extent.width - scroll.viewport.size.width).max(0.0) as f64,
                        );
                        node.set_scroll_y_max(
                            (scroll.extent.height - scroll.viewport.size.height).max(0.0) as f64,
                        );
                        node.add_action(Action::SetScrollOffset);
                    }
                }
                _ => {}
            }
            nodes.push((id, node));
        }
        let mut window = Node::new(Role::Window);
        window.set_label(title);
        window.set_bounds(bounds(prepared.bounds));
        window.set_children(
            prepared
                .semantics
                .first()
                .map(|s| vec![self.id(&s.key, 0)])
                .unwrap_or_default(),
        );
        nodes.push((root, window));
        let focus = prepared
            .focused
            .as_ref()
            .map(|key| self.id(key, 0))
            .unwrap_or(root);
        self.ids.retain(|key, _| live.contains(key));
        TreeUpdate {
            nodes,
            tree: Some(TreeInfo {
                root,
                toolkit_name: Some("Halley UI".into()),
                toolkit_version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            tree_id: TreeId::ROOT,
            focus,
        }
    }
    /// Translate an adapter request to a host-neutral action, validating text-run ownership.
    pub fn action(&self, request: ActionRequest) -> Option<AccessibleAction> {
        if request.target_tree != TreeId::ROOT {
            return None;
        }
        let (key, kind) = self.targets.get(&request.target_node)?;
        if *kind != 0 {
            return None;
        }
        match request.action {
            Action::Focus => Some(AccessibleAction::Focus(key.clone())),
            Action::Click => Some(AccessibleAction::Activate(key.clone())),
            Action::ScrollIntoView => Some(AccessibleAction::Reveal(key.clone())),
            Action::SetValue => {
                if let Some(ActionData::Value(value)) = request.data {
                    Some(AccessibleAction::SetValue {
                        key: key.clone(),
                        value: value.into(),
                    })
                } else {
                    None
                }
            }
            Action::SetScrollOffset => {
                if let Some(ActionData::SetScrollOffset(point)) = request.data {
                    Some(AccessibleAction::Scroll {
                        key: key.clone(),
                        x: point.x as f32,
                        y: point.y as f32,
                    })
                } else {
                    None
                }
            }
            Action::SetTextSelection => {
                let Some(ActionData::SetTextSelection(selection)) = request.data else {
                    return None;
                };
                let expected = (key.clone(), 1);
                if self.targets.get(&selection.anchor.node) != Some(&expected)
                    || self.targets.get(&selection.focus.node) != Some(&expected)
                {
                    return None;
                }
                let value = self.values.get(key)?;
                Some(AccessibleAction::SetSelection {
                    key: key.clone(),
                    anchor: byte_index(value, selection.anchor.character_index),
                    cursor: byte_index(value, selection.focus.character_index),
                })
            }
            _ => None,
        }
    }
}
fn char_index(value: &str, byte: usize) -> usize {
    let mut offset = 0;
    units(value)
        .iter()
        .take_while(|unit| {
            let before = offset;
            offset += unit.len();
            before < byte
        })
        .count()
}
fn byte_index(value: &str, index: usize) -> usize {
    units(value).iter().take(index).map(|unit| unit.len()).sum()
}
fn units(value: &str) -> Vec<&str> {
    value
        .graphemes(true)
        .flat_map(|grapheme| {
            if grapheme.len() <= 255 {
                vec![grapheme]
            } else {
                grapheme
                    .char_indices()
                    .map(|(i, c)| &grapheme[i..i + c.len_utf8()])
                    .collect()
            }
        })
        .collect()
}
fn bounds(rect: crate::Rect) -> accesskit::Rect {
    accesskit::Rect::new(
        rect.origin.x as f64,
        rect.origin.y as f64,
        (rect.origin.x + rect.size.width) as f64,
        (rect.origin.y + rect.size.height) as f64,
    )
}

#[cfg(all(feature = "accessibility-unix", target_os = "linux"))]
pub mod unix {
    //! One bridge per native top-level window. Adapter callbacks wake the host event loop.
    use super::*;
    use std::sync::{Arc, Mutex, mpsc};
    struct Activation(Arc<Mutex<TreeUpdate>>);
    impl accesskit::ActivationHandler for Activation {
        fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
            Some(self.0.lock().unwrap().clone())
        }
    }
    struct Actions {
        sender: mpsc::Sender<ActionRequest>,
        wake: Box<dyn Fn() + Send>,
    }
    impl accesskit::ActionHandler for Actions {
        fn do_action(&mut self, request: ActionRequest) {
            if self.sender.send(request).is_ok() {
                (self.wake)();
            }
        }
    }
    struct Deactivation;
    impl accesskit::DeactivationHandler for Deactivation {
        fn deactivate_accessibility(&mut self) {}
    }
    pub struct UnixBridge {
        adapter: accesskit_unix::Adapter,
        tree: AccessibilityTree,
        snapshot: Arc<Mutex<TreeUpdate>>,
        receiver: mpsc::Receiver<ActionRequest>,
    }
    impl UnixBridge {
        pub fn new(prepared: &PreparedView, title: &str, wake: impl Fn() + Send + 'static) -> Self {
            let mut tree = AccessibilityTree::default();
            let initial = tree.update(prepared, title);
            let snapshot = Arc::new(Mutex::new(initial));
            let (sender, receiver) = mpsc::channel();
            let adapter = accesskit_unix::Adapter::new(
                Activation(snapshot.clone()),
                Actions {
                    sender,
                    wake: Box::new(wake),
                },
                Deactivation,
            );
            Self {
                adapter,
                tree,
                snapshot,
                receiver,
            }
        }
        pub fn update(&mut self, prepared: &PreparedView, title: &str, window_focused: bool) {
            let update = self.tree.update(prepared, title);
            *self.snapshot.lock().unwrap() = update.clone();
            self.adapter.update_if_active(|| update);
            self.adapter.update_window_focus_state(window_focused);
        }
        pub fn actions(&mut self) -> Vec<AccessibleAction> {
            self.receiver
                .try_iter()
                .filter_map(|request| self.tree.action(request))
                .collect()
        }
    }
}
