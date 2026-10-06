#![cfg(feature = "accessibility")]
use halley_ui::accessibility::{
    AccessibilityTree, AccessibleAction,
    accesskit::{
        Action, ActionData, ActionRequest, NodeId, Role, TextPosition, TextSelection, TreeId,
    },
};
use halley_ui::{
    Anchor, Button, Column, InputEvent, Key, Modifiers, Rect, Scroll, TextInput, TextSystem, Theme,
    UiEvent, UiView,
};
fn view() -> UiView {
    UiView::new("dialog").anchor(Anchor::TopLeft).content(
        Scroll::new("scroll").width(260.0).height(72.0).child(
            Column::new("content")
                .child(TextInput::new("name", "Cafe\u{301} 👩‍💻").accessible_label("Cluster name"))
                .child(Button::new("create", "Create"))
                .child(Button::new("disabled", "Disabled").disabled(true)),
        ),
    )
}
fn prepared(view: &mut UiView) -> halley_ui::ui::PreparedView {
    view.prepare(
        Rect::new(0.0, 0.0, 400.0, 200.0),
        &Theme::default(),
        &mut TextSystem::new(),
    )
    .unwrap()
    .clone()
}
fn request(action: Action, id: NodeId, data: Option<ActionData>) -> ActionRequest {
    ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: id,
        data,
    }
}
#[test]
fn tree_is_valid_and_exposes_names_values_focus_selection_and_disabled_state() {
    let mut view = view();
    let scene = prepared(&mut view);
    let mut tree = AccessibilityTree::default();
    let update = tree.update(&scene, "Create cluster");
    let consumer = accesskit_consumer::Tree::new(update.clone(), true);
    assert_eq!(consumer.state().root().role(), Role::Window);
    let (input_id, input) = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::TextInput)
        .unwrap();
    assert_eq!(input.label(), Some("Cluster name"));
    assert_eq!(input.value(), Some("Cafe\u{301} 👩‍💻"));
    assert!(input.supports_action(Action::SetValue));
    let selection = input.text_selection().unwrap();
    let (_, run) = update
        .nodes
        .iter()
        .find(|(id, _)| *id == selection.focus.node)
        .unwrap();
    assert_eq!(run.character_lengths(), &[1, 1, 1, 3, 1, 11]);
    assert_eq!(selection.focus.character_index, 6);
    let disabled = update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Disabled"))
        .unwrap();
    assert!(disabled.1.is_disabled());
    assert!(!disabled.1.supports_action(Action::Click));
    let action = tree
        .action(request(Action::Focus, *input_id, None))
        .unwrap();
    view.handle_accessibility(action);
    let scene = prepared(&mut view);
    let update = tree.update(&scene, "Create cluster");
    assert_eq!(update.focus, *input_id);
    let _ = accesskit_consumer::Tree::new(update, true);
}
#[test]
fn accessible_editing_and_activation_use_the_same_widget_state_and_actions() {
    let mut view = view();
    let scene = prepared(&mut view);
    let mut tree = AccessibilityTree::default();
    let update = tree.update(&scene, "Create cluster");
    let input_id = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::TextInput)
        .unwrap()
        .0;
    let create = update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Create"))
        .unwrap()
        .0;
    let action = tree
        .action(request(
            Action::SetValue,
            input_id,
            Some(ActionData::Value("New name".into())),
        ))
        .unwrap();
    assert!(
        view.handle_accessibility(action)
            .contains(&UiEvent::TextChanged {
                key: "name".into(),
                value: "New name".into()
            })
    );
    let action = tree.action(request(Action::Click, create, None)).unwrap();
    assert_eq!(
        view.handle_accessibility(action),
        vec![UiEvent::Activate(halley_ui::ActionId::new("create"))]
    );
    view.handle_accessibility(AccessibleAction::Focus("name".into()));
    let scene = prepared(&mut view);
    let update = tree.update(&scene, "Create cluster");
    let input = update.nodes.iter().find(|(id, _)| *id == input_id).unwrap();
    let run = input.1.children()[0];
    let action = tree
        .action(request(
            Action::SetTextSelection,
            input_id,
            Some(ActionData::SetTextSelection(TextSelection {
                anchor: TextPosition {
                    node: run,
                    character_index: 0,
                },
                focus: TextPosition {
                    node: run,
                    character_index: 3,
                },
            })),
        ))
        .unwrap();
    view.handle_accessibility(action);
    assert_eq!(view.text_selection("name"), Some((0, 3)));
    let bad = tree.action(request(
        Action::SetTextSelection,
        input_id,
        Some(ActionData::SetTextSelection(TextSelection {
            anchor: TextPosition {
                node: create,
                character_index: 0,
            },
            focus: TextPosition {
                node: run,
                character_index: 3,
            },
        })),
    ));
    assert!(bad.is_none());
}
#[test]
fn accessible_node_ids_remain_stable_and_removed_controls_cannot_act() {
    let mut view = view();
    let scene = prepared(&mut view);
    let mut tree = AccessibilityTree::default();
    let first = tree.update(&scene, "Create cluster");
    view.handle_event(InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers::default(),
    });
    view.handle_event(InputEvent::Text("!".into()));
    let scene = prepared(&mut view);
    let second = tree.update(&scene, "Create cluster");
    let ids = |update: &halley_ui::accessibility::accesskit::TreeUpdate| {
        update.nodes.iter().map(|(id, _)| *id).collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), ids(&second));
    let create = first
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Create"))
        .unwrap()
        .0;
    view.set_content(TextInput::new("name", ""));
    let scene = prepared(&mut view);
    tree.update(&scene, "Create cluster");
    assert!(tree.action(request(Action::Click, create, None)).is_none());
}
