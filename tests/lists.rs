use halley_ui::ui::{LayoutError, PaintItem, PreparedView};
use halley_ui::{
    ActionId, Anchor, Button, Column, InputEvent, InteractionError, Key, Label, List, ListItem,
    Modifiers, Point, Rect, TextInput, TextSystem, Theme, UiEvent, UiView,
};

fn content(order: &[usize]) -> Column {
    let mut list = List::new("results")
        .accessible_label("Results")
        .selected("item-0")
        .width(240.0)
        .height(108.0)
        .gap(4.0);
    for i in order {
        list = list.child(
            ListItem::new(format!("item-{i}"), format!("Result {i}"))
                .action(ActionId::new(format!("open-{i}")))
                .height(32.0)
                .disabled(*i == 3 || *i == 7),
        );
    }
    Column::new("form")
        .gap(8.0)
        .child(TextInput::new("query", ""))
        .child(list)
        .child(Button::new("disabled", "Disabled").disabled(true))
        .child(Button::new("done", "Done"))
}
fn view() -> UiView {
    UiView::new("demo")
        .anchor(Anchor::TopLeft)
        .content(content(&(0..12).collect::<Vec<_>>()))
}
fn prepare(view: &mut UiView) -> PreparedView {
    view.prepare_with_measure(
        Rect::new(20.0, 30.0, 400.0, 400.0),
        &Theme::default(),
        |_, text| halley_ui::Size::new(text.len() as f32 * 8.0, 16.0),
    )
    .unwrap()
    .clone()
}
fn key(view: &mut UiView, key: Key) -> Vec<UiEvent> {
    view.handle_event(InputEvent::KeyDown {
        key,
        modifiers: Modifiers::default(),
    })
}
fn assert_visible(scene: &PreparedView, item: &str) {
    let control = scene.controls.iter().find(|c| c.key == item).unwrap();
    assert_eq!(
        control.rect.intersection(control.clip),
        control.rect,
        "{control:?}"
    );
}

#[test]
fn focus_requests_validate_targets_and_tab_treats_the_list_as_one_stop() {
    let mut view = view();
    assert_eq!(
        view.focus("query").unwrap_err(),
        InteractionError::NotPrepared
    );
    prepare(&mut view);
    assert_eq!(
        view.focus("missing").unwrap_err(),
        InteractionError::UnknownControl("missing".into())
    );
    assert_eq!(
        view.focus("disabled").unwrap_err(),
        InteractionError::Disabled("disabled".into())
    );
    assert_eq!(
        view.focus("query").unwrap(),
        vec![UiEvent::FocusChanged(Some("query".into()))]
    );
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("results"));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("done"));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("query"));
    view.handle_event(InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers {
            shift: true,
            ..Default::default()
        },
    });
    assert_eq!(view.focused(), Some("done"));
    assert_eq!(view.clear_focus(), vec![UiEvent::FocusChanged(None)]);
    assert_eq!(view.focused(), None);
}

#[test]
fn selection_navigation_preserves_search_focus_skips_disabled_rows_and_clamps() {
    let mut view = view();
    prepare(&mut view);
    view.focus("query").unwrap();
    assert_eq!(
        view.navigate_list("results", Key::Down).unwrap(),
        vec![UiEvent::SelectionChanged {
            key: "results".into(),
            selected: Some("item-1".into()),
        }]
    );
    view.navigate_list("results", Key::Down).unwrap();
    view.navigate_list("results", Key::Down).unwrap();
    assert_eq!(view.selected("results"), Some("item-4"));
    assert_eq!(view.focused(), Some("query"));
    view.navigate_list("results", Key::End).unwrap();
    assert!(view.navigate_list("results", Key::Down).unwrap().is_empty());
    assert_eq!(view.selected("results"), Some("item-11"));
    assert_visible(&prepare(&mut view), "item-11");
    view.navigate_list("results", Key::Home).unwrap();
    assert!(view.navigate_list("results", Key::Up).unwrap().is_empty());
    assert_eq!(view.selected("results"), Some("item-0"));
    assert_visible(&prepare(&mut view), "item-0");
    assert_eq!(
        view.navigate_list("results", Key::Left).unwrap_err(),
        InteractionError::InvalidNavigation
    );
    assert!(matches!(
        view.select("results", Some("item-3")),
        Err(InteractionError::InvalidItem { .. })
    ));
    assert!(matches!(
        view.select("results", Some("query")),
        Err(InteractionError::InvalidItem { .. })
    ));
    assert_eq!(
        view.activate_selected("results").unwrap(),
        vec![UiEvent::Activate(ActionId::new("open-0"))]
    );
}

#[test]
fn focused_list_supports_arrows_pages_home_end_and_enter_with_geometry_based_paging() {
    let mut view = view();
    prepare(&mut view);
    view.focus("results").unwrap();
    key(&mut view, Key::PageDown);
    assert_eq!(view.selected("results"), Some("item-4"));
    assert_visible(&prepare(&mut view), "item-4");
    key(&mut view, Key::PageUp);
    assert_eq!(view.selected("results"), Some("item-1"));
    key(&mut view, Key::End);
    assert_visible(&prepare(&mut view), "item-11");
    assert_eq!(
        key(&mut view, Key::Enter),
        vec![UiEvent::Activate(ActionId::new("open-11"))]
    );
    key(&mut view, Key::Home);
    key(&mut view, Key::Down);
    assert_eq!(view.selected("results"), Some("item-1"));
    assert_eq!(view.focused(), Some("results"));
}

#[test]
fn repeated_navigation_before_redraw_and_wheel_scrolling_do_not_fight_reveal() {
    let mut view = view();
    prepare(&mut view);
    for _ in 0..9 {
        view.navigate_list("results", Key::Down).unwrap();
    }
    assert_eq!(view.selected("results"), Some("item-11"));
    let scene = prepare(&mut view);
    assert_visible(&scene, "item-11");
    let viewport = scene
        .scrolls
        .iter()
        .find(|s| s.key == "results")
        .unwrap()
        .viewport;
    view.handle_event(InputEvent::Wheel {
        position: Point::new(viewport.origin.x + 5.0, viewport.origin.y + 5.0),
        delta: Point::new(0.0, -10000.0),
    });
    let scene = prepare(&mut view);
    assert_eq!(scene.scrolls[0].offset.y, 0.0);
    assert_eq!(view.selected("results"), Some("item-11"));
    view.focus("results").unwrap();
    assert_visible(&prepare(&mut view), "item-11");
}

#[test]
fn stable_selection_survives_reordering_and_clears_when_removed_or_disabled() {
    let mut view = view();
    prepare(&mut view);
    view.select("results", Some("item-10")).unwrap();
    view.set_content(content(&[10, 1, 0, 2]));
    let scene = prepare(&mut view);
    assert_eq!(view.selected("results"), Some("item-10"));
    assert_visible(&scene, "item-10");
    view.set_content(content(&[1, 0, 2]));
    prepare(&mut view);
    assert_eq!(view.selected("results"), None);
    view.set_content(
        List::new("results")
            .selected("item-0")
            .child(ListItem::new("item-0", "Zero").disabled(true)),
    );
    prepare(&mut view);
    assert_eq!(view.selected("results"), None);
    view.set_content(Label::new("gone", "List removed"));
    prepare(&mut view);
    view.set_content(
        List::new("results")
            .selected("fresh")
            .child(ListItem::new("fresh", "Fresh")),
    );
    prepare(&mut view);
    assert_eq!(view.selected("results"), Some("fresh"));
}

#[test]
fn translated_pointer_selection_and_activation_obey_clips_capture_and_snapshot_ownership() {
    let mut view = view();
    let scene = prepare(&mut view).translated(Point::new(100.0, 40.0));
    let rect = scene.rects["item-1"];
    let p = Point::new(rect.origin.x + 5.0, rect.origin.y + 5.0);
    let events = view.handle_event_in(
        &scene,
        InputEvent::PointerDown {
            position: p,
            shift: false,
        },
    );
    assert!(events.contains(&UiEvent::SelectionChanged {
        key: "results".into(),
        selected: Some("item-1".into())
    }));
    assert_eq!(view.focused(), Some("results"));
    assert_eq!(
        view.handle_event_in(&scene, InputEvent::PointerUp { position: p }),
        vec![UiEvent::Activate(ActionId::new("open-1"))]
    );
    let offscreen = scene.rects["item-11"].origin;
    view.handle_event_in(
        &scene,
        InputEvent::PointerDown {
            position: offscreen,
            shift: false,
        },
    );
    assert_eq!(view.selected("results"), Some("item-1"));
    view.handle_event_in(
        &scene,
        InputEvent::PointerDown {
            position: p,
            shift: false,
        },
    );
    view.handle_event(InputEvent::PointerCancel);
    assert!(
        view.handle_event_in(&scene, InputEvent::PointerUp { position: p })
            .is_empty()
    );
    view.set_content(content(&[2, 1, 0]));
    assert_eq!(
        view.select("results", Some("item-2")).unwrap_err(),
        InteractionError::NotPrepared
    );
    assert!(
        view.handle_event_in(
            &scene,
            InputEvent::PointerDown {
                position: p,
                shift: false
            }
        )
        .is_empty()
    );
}

#[test]
fn empty_and_disabled_lists_do_not_select_or_activate_and_focus_is_independent() {
    let mut view = UiView::new("empty").content(List::new("results").width(200.0).height(100.0));
    prepare(&mut view);
    view.focus("results").unwrap();
    assert!(view.navigate_list("results", Key::Down).unwrap().is_empty());
    assert!(view.activate_selected("results").unwrap().is_empty());
    view.set_content(List::new("results").child(ListItem::new("locked", "Locked").disabled(true)));
    prepare(&mut view);
    assert!(view.navigate_list("results", Key::Up).unwrap().is_empty());
    view.set_content(
        List::new("results")
            .disabled(true)
            .child(ListItem::new("one", "One")),
    );
    prepare(&mut view);
    assert_eq!(view.focused(), None);
    assert!(matches!(
        view.focus("one"),
        Err(InteractionError::Disabled(_))
    ));
    assert!(matches!(
        view.navigate_list("results", Key::Down),
        Err(InteractionError::Disabled(_))
    ));
    assert!(matches!(
        view.activate_selected("results"),
        Err(InteractionError::Disabled(_))
    ));
}

#[test]
fn custom_options_keep_one_control_and_selection_is_painted_without_keyboard_focus() {
    let theme = Theme::default();
    let mut view = UiView::new("custom").content(
        List::new("results").selected("one").child(
            ListItem::new("one", "Accessible title")
                .child(Column::new("row").child(Label::new("title", "Visible title"))),
        ),
    );
    let scene = prepare(&mut view);
    assert_eq!(scene.controls.len(), 2);
    assert_eq!(view.focused(), None);
    assert!(
        scene
            .items
            .iter()
            .any(|p| matches!(p, PaintItem::Card { key, style, .. }
        if key == "custom/one" && style.fill == theme.button_pressed))
    );
    assert!(
        !scene
            .items
            .iter()
            .any(|p| matches!(p, PaintItem::Text { text, .. } if text == "Accessible title"))
    );
    assert!(
        scene
            .items
            .iter()
            .any(|p| matches!(p, PaintItem::Text { text, .. } if text == "Visible title"))
    );
    view.set_content(
        List::new("results")
            .child(ListItem::new("one", "Invalid").child(Button::new("nested", "Nested"))),
    );
    assert!(matches!(
        view.prepare(
            Rect::new(0.0, 0.0, 400.0, 200.0),
            &theme,
            &mut TextSystem::new()
        ),
        Err(LayoutError::InvalidListItem(_))
    ));
}

#[test]
fn direct_focus_cancels_composition_pending_paste_and_capture() {
    let mut view = view();
    prepare(&mut view);
    view.focus("query").unwrap();
    let events = view.handle_event(InputEvent::KeyDown {
        key: Key::V,
        modifiers: Modifiers {
            control: true,
            ..Default::default()
        },
    });
    let request = events
        .iter()
        .find_map(|e| match e {
            UiEvent::ClipboardRead { request } => Some(*request),
            _ => None,
        })
        .unwrap();
    view.handle_event(InputEvent::Preedit {
        text: "日本".into(),
        cursor: None,
    });
    view.focus("item-10").unwrap();
    assert_eq!(view.focused(), Some("results"));
    assert_eq!(view.selected("results"), Some("item-10"));
    assert_visible(&prepare(&mut view), "item-10");
    view.focus("query").unwrap();
    view.handle_event(InputEvent::Paste {
        request,
        text: "Stale".into(),
    });
    assert_eq!(view.text_value("query"), Some(""));
    assert!(
        !prepare(&mut view)
            .items
            .iter()
            .any(|p| matches!(p, PaintItem::Text { text, .. } if text == "日本"))
    );
}

#[test]
fn variable_height_paging_and_reveal_work_through_nested_scroll_clips() {
    use halley_ui::Scroll;
    let mut list = List::new("results").width(220.0).height(120.0).gap(5.0);
    for (i, height) in [20.0, 80.0, 30.0, 70.0, 24.0].into_iter().enumerate() {
        list = list.child(
            ListItem::new(format!("row-{i}"), format!("Row {i}"))
                .height(height)
                .padding(0.0),
        );
    }
    let mut view = UiView::new("nested").anchor(Anchor::TopLeft).content(
        Scroll::new("outer").width(240.0).height(160.0).child(
            Column::new("column")
                .child(Label::new("spacer", "Above the list").height(100.0))
                .child(list),
        ),
    );
    prepare(&mut view);
    view.focus("results").unwrap();
    view.navigate_list("results", Key::Home).unwrap();
    view.navigate_list("results", Key::PageDown).unwrap();
    assert_eq!(view.selected("results"), Some("row-3"));
    assert_visible(&prepare(&mut view), "row-3");
    view.navigate_list("results", Key::End).unwrap();
    assert_visible(&prepare(&mut view), "row-4");
    view.navigate_list("results", Key::PageUp).unwrap();
    assert_eq!(view.selected("results"), Some("row-1"));
    assert_visible(&prepare(&mut view), "row-1");
}

#[test]
fn clearing_a_queued_selection_cancels_reveal_and_selection_changes_cancel_click_capture() {
    let mut view = view();
    let scene = prepare(&mut view);
    view.select("results", Some("item-11")).unwrap();
    view.select("results", None).unwrap();
    assert_eq!(prepare(&mut view).scrolls[0].offset.y, 0.0);
    assert!(view.activate_selected("results").unwrap().is_empty());
    let p = scene.rects["item-1"].origin;
    let p = Point::new(p.x + 5.0, p.y + 5.0);
    view.handle_event(InputEvent::PointerDown {
        position: p,
        shift: false,
    });
    view.navigate_list("results", Key::Down).unwrap();
    assert!(
        view.handle_event(InputEvent::PointerUp { position: p })
            .is_empty()
    );
    assert_eq!(view.selected("results"), Some("item-2"));
}
