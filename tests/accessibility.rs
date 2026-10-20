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
fn validation_state_is_exposed_and_can_clear_without_disabling_the_editor() {
    let content = |invalid| {
        TextInput::new("name", "Original")
            .accessible_label("Name")
            .invalid(invalid)
    };
    let mut view = UiView::new("validation").content(content(true));
    let scene = prepared(&mut view);
    let mut tree = AccessibilityTree::default();
    let update = tree.update(&scene, "Edit name");
    let (id, input) = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::TextInput)
        .unwrap();
    let id = *id;
    assert_eq!(
        input.invalid(),
        Some(halley_ui::accessibility::accesskit::Invalid::True)
    );
    assert!(!input.is_disabled());
    assert!(input.supports_action(Action::SetValue));
    let _ = accesskit_consumer::Tree::new(update, true);
    view.focus("name").unwrap();
    view.set_text_value("name", "Corrected");
    view.set_content(content(false));
    let scene = prepared(&mut view);
    let update = tree.update(&scene, "Edit name");
    let input = &update.nodes.iter().find(|(node, _)| *node == id).unwrap().1;
    assert_eq!(input.invalid(), None);
    assert_eq!(input.value(), Some("Corrected"));
    assert_eq!(update.focus, id);
    let _ = accesskit_consumer::Tree::new(update, true);
}

#[test]
fn list_options_expose_selection_active_descendant_and_accessible_focus_and_activation() {
    use halley_ui::{List, ListItem};
    let content = |reverse| {
        let mut list = List::new("results")
            .accessible_label("Applications")
            .width(240.0)
            .height(60.0);
        for i in if reverse {
            vec![4, 3, 2, 1, 0]
        } else {
            vec![0, 1, 2, 3, 4]
        } {
            list = list.child(
                ListItem::new(format!("option-{i}"), format!("App {i}"))
                    .height(30.0)
                    .disabled(i == 2),
            );
        }
        Column::new("content")
            .child(TextInput::new("search", ""))
            .child(list)
    };
    let mut view = UiView::new("list").content(content(false));
    let mut tree = AccessibilityTree::default();
    let scene = prepared(&mut view);
    let update = tree.update(&scene, "Applications");
    let _ = accesskit_consumer::Tree::new(update.clone(), true);
    let list_id = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::ListBox)
        .unwrap()
        .0;
    let item_id = update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("App 4"))
        .unwrap()
        .0;
    let disabled = update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("App 2"))
        .unwrap();
    assert!(disabled.1.is_disabled());
    assert!(!disabled.1.supports_action(Action::Focus));
    assert!(!disabled.1.supports_action(Action::Click));
    assert!(
        view.handle_accessibility(AccessibleAction::Focus("option-2".into()))
            .is_empty()
    );
    let action = tree.action(request(Action::Focus, item_id, None)).unwrap();
    let events = view.handle_accessibility(action);
    assert!(events.contains(&UiEvent::SelectionChanged {
        key: "results".into(),
        selected: Some("option-4".into())
    }));
    assert_eq!(view.focused(), Some("results"));
    let scene = prepared(&mut view);
    assert!(scene.scrolls[0].offset.y > 0.0);
    let update = tree.update(&scene, "Applications");
    assert_eq!(update.focus, list_id);
    let list = update.nodes.iter().find(|(id, _)| *id == list_id).unwrap();
    assert_eq!(list.1.active_descendant(), Some(item_id));
    let item = update.nodes.iter().find(|(id, _)| *id == item_id).unwrap();
    assert_eq!(item.1.role(), Role::ListBoxOption);
    assert_eq!(item.1.is_selected(), Some(true));
    let _ = accesskit_consumer::Tree::new(update, true);
    view.focus("search").unwrap();
    let action = tree.action(request(Action::Click, item_id, None)).unwrap();
    assert!(
        view.handle_accessibility(action)
            .contains(&UiEvent::Activate(halley_ui::ActionId::new("option-4")))
    );
    assert_eq!(view.focused(), Some("search"));
    view.set_content(content(true));
    let scene = prepared(&mut view);
    let update = tree.update(&scene, "Applications");
    assert!(
        update
            .nodes
            .iter()
            .any(|(id, n)| *id == item_id && n.is_selected() == Some(true))
    );
    let _ = accesskit_consumer::Tree::new(update, true);
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
