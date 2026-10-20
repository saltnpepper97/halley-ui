#![doc = include_str!("../docs/components/list.md")]
//! Retained single-selection lists and programmatic focus.
use super::*;
use crate::input::{Key, UiEvent};

/// A scrollable, single-selection list. Items use stable widget keys.
#[derive(Clone, Debug)]
pub struct List(Widget);
impl List {
    pub fn new(key: impl Into<String>) -> Self {
        Self(Widget::new(
            key,
            Kind::List {
                initial_selection: None,
                selection_color: None,
                disabled: false,
            },
        ))
    }
    pub fn child(mut self, item: ListItem) -> Self {
        self.0.children.push(item.0);
        self
    }
    pub fn gap(mut self, value: f32) -> Self {
        self.0.layout.gap = Some(value.max(0.0));
        self
    }
    /// Seeds selection once per list key; use `UiView::select` for later changes.
    pub fn selected(mut self, key: impl Into<String>) -> Self {
        if let Kind::List {
            initial_selection, ..
        } = &mut self.0.kind
        {
            *initial_selection = Some(key.into());
        }
        self
    }
    pub fn selection_color(mut self, value: Color) -> Self {
        if let Kind::List {
            selection_color, ..
        } = &mut self.0.kind
        {
            *selection_color = Some(value);
        }
        self
    }
    /// Shared partial overrides for this list's options; item overrides win.
    pub fn item_style(mut self, style: StyleSheet) -> Self {
        self.0.item_style = Some(Arc::new(style));
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        if let Kind::List { disabled, .. } = &mut self.0.kind {
            *disabled = value;
        }
        self
    }
    control_style_methods!();
    layout_methods!();
}
impl From<List> for Widget {
    fn from(value: List) -> Self {
        value.0
    }
}

/// An option with a default text label or custom, noninteractive child content.
#[derive(Clone, Debug)]
pub struct ListItem(Widget);
impl ListItem {
    pub fn new(key: impl Into<String>, text: impl Into<String>) -> Self {
        let key = key.into();
        let widget = Widget::new(
            key.clone(),
            Kind::ListItem {
                text: text.into(),
                action: ActionId::new(key),
                disabled: false,
            },
        );
        Self(widget)
    }
    /// Custom content replaces the painted label; the label remains the accessible name.
    pub fn child(mut self, value: impl Into<Widget>) -> Self {
        self.0.children.push(value.into());
        self
    }
    pub fn action(mut self, value: ActionId) -> Self {
        if let Kind::ListItem { action, .. } = &mut self.0.kind {
            *action = value;
        }
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        if let Kind::ListItem { disabled, .. } = &mut self.0.kind {
            *disabled = value;
        }
        self
    }
    control_style_methods!();
    layout_methods!();
}
impl From<ListItem> for Widget {
    fn from(value: ListItem) -> Self {
        value.0
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum InteractionError {
    #[error("control is not a slider: {0}")]
    NotSlider(String),
    #[error("numeric value must be finite")]
    InvalidNumericValue,
    #[error("control is not a checkbox or switch: {0}")]
    NotCheckable(String),
    #[error("control is not a radio group: {0}")]
    NotRadioGroup(String),
    #[error("option {option} does not belong to radio group {group}")]
    InvalidRadioOption { group: String, option: String },
    #[error("prepare the current content before requesting focus or selection")]
    NotPrepared,
    #[error("unknown control: {0}")]
    UnknownControl(String),
    #[error("control is disabled: {0}")]
    Disabled(String),
    #[error("control is not a list: {0}")]
    NotList(String),
    #[error("item {item} is not an enabled option in list {list}")]
    InvalidItem { list: String, item: String },
    #[error("list navigation requires Up, Down, PageUp, PageDown, Home, or End")]
    InvalidNavigation,
}

pub(super) fn validate(
    widget: &Widget,
    list_parent: bool,
    inside_item: bool,
) -> Result<(), LayoutError> {
    if (list_parent && !matches!(widget.kind, Kind::ListItem { .. }))
        || (!list_parent && matches!(widget.kind, Kind::ListItem { .. }))
        || (inside_item
            && matches!(
                widget.kind,
                Kind::Button { .. }
                    | Kind::TextInput { .. }
                    | Kind::List { .. }
                    | Kind::Scroll
                    | Kind::Slider { .. }
                    | Kind::Checkbox { .. }
                    | Kind::Switch { .. }
                    | Kind::RadioGroup { .. }
                    | Kind::RadioOption { .. }
            ))
    {
        return Err(LayoutError::InvalidListItem(widget.key.clone()));
    }
    for child in &widget.children {
        validate(
            child,
            matches!(widget.kind, Kind::List { .. }),
            inside_item || matches!(widget.kind, Kind::ListItem { .. }),
        )?;
    }
    Ok(())
}

impl UiView {
    pub(super) fn sync_lists(&mut self) {
        fn visit(
            widget: &Widget,
            state: &mut crate::input::State,
            live: &mut HashSet<String>,
            reveal: bool,
        ) {
            if let Kind::List {
                initial_selection, ..
            } = &widget.kind
            {
                live.insert(widget.key.clone());
                let selected = state
                    .lists
                    .entry(widget.key.clone())
                    .or_insert_with(|| initial_selection.clone());
                if selected.as_ref().is_some_and(|key| {
                    !widget.children.iter().any(|item| {
                        &item.key == key
                            && matches!(
                                item.kind,
                                Kind::ListItem {
                                    disabled: false,
                                    ..
                                }
                            )
                    })
                }) {
                    *selected = None;
                }
                if reveal && let Some(key) = selected {
                    state.pending_reveal.push(key.clone());
                }
            }
            for child in &widget.children {
                visit(child, state, live, reveal);
            }
        }
        let mut live = HashSet::new();
        visit(
            &self.root,
            &mut self.state,
            &mut live,
            self.input_scene.is_none(),
        );
        self.state.lists.retain(|key, _| live.contains(key));
    }

    /// Request focus after preparation. List options focus their owner and select the option.
    /// This changes local widget focus; native window activation remains host-owned.
    pub fn focus(&mut self, key: &str) -> Result<Vec<UiEvent>, InteractionError> {
        self.sync_state();
        let scene = self
            .input_scene
            .clone()
            .ok_or(InteractionError::NotPrepared)?;
        let control = scene
            .controls
            .iter()
            .find(|c| c.key == key)
            .ok_or_else(|| InteractionError::UnknownControl(key.into()))?;
        if control.disabled {
            return Err(InteractionError::Disabled(key.into()));
        }
        let mut out = Vec::new();
        let target = if matches!(control.kind, ControlKind::RadioGroup) {
            forms::radio_entry(key, &scene.controls, &self.state)
                .map(|c| c.key.clone())
                .ok_or_else(|| InteractionError::Disabled(key.into()))?
        } else if let ControlKind::ListItem { list, .. } = &control.kind {
            self.select_in(list, Some(key), &scene, &mut out)?;
            list.clone()
        } else {
            key.into()
        };
        self.change_focus(Some(target), &mut out);
        self.invalidate();
        Ok(out)
    }
    /// Clear local focus, composition, capture, and pending clipboard requests.
    pub fn clear_focus(&mut self) -> Vec<UiEvent> {
        let mut out = Vec::new();
        self.change_focus(None, &mut out);
        self.invalidate();
        out
    }
    pub fn selected(&self, list: &str) -> Option<&str> {
        self.state.lists.get(list).and_then(|key| key.as_deref())
    }
    /// Set or clear selection without moving keyboard focus. Reveals the selected row.
    pub fn select(
        &mut self,
        list: &str,
        item: Option<&str>,
    ) -> Result<Vec<UiEvent>, InteractionError> {
        self.sync_state();
        let scene = self
            .input_scene
            .clone()
            .ok_or(InteractionError::NotPrepared)?;
        let mut out = Vec::new();
        self.select_in(list, item, &scene, &mut out)?;
        self.invalidate();
        Ok(out)
    }
    /// Route list navigation while retaining focus in another control, such as a search field.
    pub fn navigate_list(
        &mut self,
        list: &str,
        key: Key,
    ) -> Result<Vec<UiEvent>, InteractionError> {
        self.sync_state();
        let scene = self
            .input_scene
            .clone()
            .ok_or(InteractionError::NotPrepared)?;
        let mut out = Vec::new();
        self.navigate_list_in(list, key, &scene, &mut out)?;
        self.invalidate();
        Ok(out)
    }
    /// Activate the current enabled option without moving keyboard focus.
    pub fn activate_selected(&self, list: &str) -> Result<Vec<UiEvent>, InteractionError> {
        let scene = self
            .input_scene
            .as_ref()
            .ok_or(InteractionError::NotPrepared)?;
        self.list_control(list, scene)?;
        Ok(self.selected(list).and_then(|selected| scene.controls.iter().find(|c|
            c.key == selected && !c.disabled && matches!(&c.kind, ControlKind::ListItem { list: owner, .. } if owner == list)))
            .and_then(|c| match &c.kind {
                ControlKind::ListItem { action, .. } => Some(vec![UiEvent::Activate(action.clone())]),
                _ => None,
            }).unwrap_or_default())
    }
    fn list_control<'a>(
        &self,
        list: &str,
        scene: &'a interaction::InputScene,
    ) -> Result<&'a ControlRegion, InteractionError> {
        let control = scene
            .controls
            .iter()
            .find(|c| c.key == list)
            .ok_or_else(|| InteractionError::UnknownControl(list.into()))?;
        if !matches!(control.kind, ControlKind::List) {
            return Err(InteractionError::NotList(list.into()));
        }
        if control.disabled {
            return Err(InteractionError::Disabled(list.into()));
        }
        Ok(control)
    }
    pub(super) fn select_in(
        &mut self,
        list: &str,
        item: Option<&str>,
        scene: &interaction::InputScene,
        out: &mut Vec<UiEvent>,
    ) -> Result<(), InteractionError> {
        self.list_control(list, scene)?;
        if let Some(item) = item
            && !scene.controls.iter().any(|c| {
                c.key == item
                    && !c.disabled
                    && matches!(&c.kind, ControlKind::ListItem { list: owner, .. } if owner == list)
            })
        {
            return Err(InteractionError::InvalidItem {
                list: list.into(),
                item: item.into(),
            });
        }
        if self.selected(list) != item {
            if self.state.pressed.as_ref().is_some_and(|pressed| scene.controls.iter().any(|c|
                &c.key == pressed && matches!(&c.kind, ControlKind::ListItem { list: owner, .. } if owner == list))) {
                self.state.pressed = None;
            }
            self.state
                .lists
                .insert(list.into(), item.map(str::to_owned));
            out.push(UiEvent::SelectionChanged {
                key: list.into(),
                selected: item.map(str::to_owned),
            });
        }
        // Only the final selection in a list needs revealing, including when it is cleared.
        self.state.pending_reveal.retain(|key| {
            !scene.controls.iter().any(|c| {
                &c.key == key
                    && matches!(&c.kind, ControlKind::ListItem { list: owner, .. } if owner == list)
            })
        });
        if let Some(item) = item {
            self.state.pending_reveal.push(item.into());
        }
        Ok(())
    }
    pub(super) fn navigate_list_in(
        &mut self,
        list: &str,
        key: Key,
        scene: &interaction::InputScene,
        out: &mut Vec<UiEvent>,
    ) -> Result<(), InteractionError> {
        self.list_control(list, scene)?;
        if !matches!(
            key,
            Key::Up | Key::Down | Key::PageUp | Key::PageDown | Key::Home | Key::End
        ) {
            return Err(InteractionError::InvalidNavigation);
        }
        let items: Vec<_> = scene
            .controls
            .iter()
            .filter(|c| {
                !c.disabled
                    && matches!(&c.kind, ControlKind::ListItem { list: owner, .. } if owner == list)
            })
            .collect();
        if items.is_empty() {
            return Ok(());
        }
        let current = items
            .iter()
            .position(|c| self.selected(list) == Some(c.key.as_str()));
        let index = match (key, current) {
            (Key::Home, _) | (Key::Down | Key::PageDown, None) => 0,
            (Key::End, _) | (Key::Up | Key::PageUp, None) => items.len() - 1,
            (Key::Down, Some(i)) => (i + 1).min(items.len() - 1),
            (Key::Up, Some(i)) => i.saturating_sub(1),
            (Key::PageUp | Key::PageDown, Some(i)) => {
                let height = scene
                    .scrolls
                    .iter()
                    .find(|s| s.key == list)
                    .map_or(0.0, |s| s.viewport.size.height);
                if key == Key::PageDown {
                    let target = items[i].rect.origin.y + height;
                    items
                        .iter()
                        .position(|c| c.rect.origin.y >= target)
                        .unwrap_or(items.len() - 1)
                        .max((i + 1).min(items.len() - 1))
                } else {
                    let target = items[i].rect.origin.y - height;
                    items
                        .iter()
                        .rposition(|c| c.rect.origin.y <= target)
                        .unwrap_or(0)
                        .min(i.saturating_sub(1))
                }
            }
            _ => unreachable!(),
        };
        self.select_in(list, Some(&items[index].key), scene, out)
    }
}
