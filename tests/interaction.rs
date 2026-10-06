use halley_ui::ui::PaintItem;
use halley_ui::{
    Anchor, Button, Column, InputEvent, Key, Modifiers, Point, Rect, Scroll, TextInput, TextSystem,
    Theme, UiEvent, UiView,
};
fn form() -> UiView {
    UiView::new("form").anchor(Anchor::TopLeft).content(
        Column::new("column")
            .gap(8.0)
            .child(TextInput::new("name", "Work").accessible_label("Cluster name"))
            .child(Button::new("disabled", "Unavailable").disabled(true))
            .child(Button::new("save", "Create")),
    )
}
fn prepare(view: &mut UiView, text: &mut TextSystem) -> halley_ui::ui::PreparedView {
    view.prepare(Rect::new(20.0, 30.0, 400.0, 300.0), &Theme::default(), text)
        .unwrap()
        .clone()
}
fn key(view: &mut UiView, key: Key, shift: bool, control: bool) -> Vec<UiEvent> {
    view.handle_event(InputEvent::KeyDown {
        key,
        modifiers: Modifiers {
            shift,
            control,
            alt: false,
        },
    })
}
#[test]
fn cluster_name_can_be_selected_replaced_and_submitted_without_rebuilding_widgets() {
    let mut view = form();
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    assert_eq!(view.focused(), Some("name"));
    key(&mut view, Key::A, false, true);
    let result = view.handle_event(InputEvent::Text("Cafe\u{301} 👩‍💻".into()));
    assert!(result.contains(&UiEvent::TextChanged {
        key: "name".into(),
        value: "Cafe\u{301} 👩‍💻".into()
    }));
    key(&mut view, Key::Backspace, false, false);
    assert_eq!(view.text_value("name"), Some("Cafe\u{301} "));
    key(&mut view, Key::Backspace, false, false);
    key(&mut view, Key::Backspace, false, false);
    assert_eq!(view.text_value("name"), Some("Caf"));
    assert!(
        key(&mut view, Key::Enter, false, false).contains(&UiEvent::Submit {
            key: "name".into(),
            value: "Caf".into()
        })
    );
    let scene = prepare(&mut view, &mut text);
    assert!(
        scene
            .items
            .iter()
            .any(|p| matches!(p,PaintItem::Text{text,..} if text=="Caf"))
    );
}
#[test]
fn focus_skips_disabled_controls_wraps_and_keyboard_activation_matches_click() {
    let mut view = form();
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    key(&mut view, Key::Tab, false, false);
    assert_eq!(view.focused(), Some("save"));
    let scene = prepare(&mut view, &mut text);
    assert!(
        scene
            .items
            .iter()
            .any(|p| matches!(p,PaintItem::Card {key,..} if key=="form/save/focus"))
    );
    key(&mut view, Key::Space, false, false);
    assert_eq!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space }),
        vec![UiEvent::Activate(halley_ui::ActionId::new("save"))]
    );
    key(&mut view, Key::Tab, false, false);
    assert_eq!(view.focused(), Some("name"));
    key(&mut view, Key::Tab, true, false);
    assert_eq!(view.focused(), Some("save"));
    let scene = prepare(&mut view, &mut text).translated(Point::new(100.0, -10.0));
    let r = scene.rects["save"];
    let p = Point::new(r.origin.x + 5.0, r.origin.y + 5.0);
    view.handle_event_in(
        &scene,
        InputEvent::PointerDown {
            position: p,
            shift: false,
        },
    );
    assert_eq!(
        view.handle_event_in(&scene, InputEvent::PointerUp { position: p }),
        vec![UiEvent::Activate(halley_ui::ActionId::new("save"))]
    );
}
#[test]
fn clipboard_replies_cannot_paste_into_a_different_control_or_selection() {
    let mut view = form();
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    key(&mut view, Key::A, false, true);
    assert_eq!(
        key(&mut view, Key::C, false, true),
        vec![UiEvent::ClipboardWrite("Work".into())]
    );
    let request = match key(&mut view, Key::V, false, true)[0] {
        UiEvent::ClipboardRead { request } => request,
        _ => panic!(),
    };
    key(&mut view, Key::Right, false, false);
    view.handle_event(InputEvent::Paste {
        request,
        text: "Wrong".into(),
    });
    assert_eq!(view.text_value("name"), Some("Work"));
    key(&mut view, Key::A, false, true);
    let request = match key(&mut view, Key::V, false, true)[0] {
        UiEvent::ClipboardRead { request } => request,
        _ => panic!(),
    };
    view.handle_event(InputEvent::Paste {
        request,
        text: "New\nName\r\t".into(),
    });
    assert_eq!(view.text_value("name"), Some("NewName"));
    let request = match key(&mut view, Key::V, false, true)[0] {
        UiEvent::ClipboardRead { request } => request,
        _ => panic!(),
    };
    key(&mut view, Key::Tab, false, false);
    view.handle_event(InputEvent::Paste {
        request,
        text: "Wrong".into(),
    });
    assert_eq!(view.text_value("name"), Some("NewName"));
}
#[test]
fn ime_preedit_is_temporary_replaces_selection_on_commit_and_cancels_on_blur() {
    let mut view = form();
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    key(&mut view, Key::A, false, true);
    view.handle_event(InputEvent::Preedit {
        text: "日本".into(),
        cursor: Some(3),
    });
    assert_eq!(view.text_value("name"), Some("Work"));
    let scene = prepare(&mut view, &mut text);
    assert!(
        scene
            .items
            .iter()
            .any(|p| matches!(p,PaintItem::Text{text,..} if text=="日本"))
    );
    assert!(matches!(
        view.ime_state(),
        UiEvent::Ime {
            key: Some(_),
            cursor: 4,
            anchor: 0,
            ..
        }
    ));
    view.handle_event(InputEvent::Text("日本語".into()));
    assert_eq!(view.text_value("name"), Some("日本語"));
    view.handle_event(InputEvent::Preedit {
        text: "仮".into(),
        cursor: None,
    });
    view.handle_event(InputEvent::WindowFocus(false));
    view.handle_event(InputEvent::Text("Ignored".into()));
    assert_eq!(view.text_value("name"), Some("日本語"));
    assert!(matches!(view.ime_state(), UiEvent::Ime { key: None, .. }));
    view.handle_event(InputEvent::WindowFocus(true));
    let scene = prepare(&mut view, &mut text);
    assert!(
        !scene
            .items
            .iter()
            .any(|p| matches!(p,PaintItem::Text{text,..} if text.contains('仮')))
    );
}
#[test]
fn scrolling_clips_hits_and_tab_reveals_an_offscreen_button() {
    let mut view = UiView::new("list").anchor(Anchor::TopLeft).content(
        Scroll::new("scroll").width(180.0).height(72.0).child(
            Column::new("items")
                .gap(4.0)
                .child(Button::new("one", "One").height(32.0))
                .child(Button::new("two", "Two").height(32.0))
                .child(Button::new("three", "Three").height(32.0))
                .child(Button::new("four", "Four").height(32.0)),
        ),
    );
    let mut text = TextSystem::new();
    let scene = prepare(&mut view, &mut text);
    assert!(
        scene.scrolls[0].extent.height > scene.scrolls[0].viewport.size.height,
        "{scene:?}"
    );
    let r = scene.rects["four"];
    assert!(
        scene
            .hit(Point::new(r.origin.x + 5.0, r.origin.y + 5.0))
            .is_none()
    );
    for _ in 0..4 {
        key(&mut view, Key::Tab, false, false);
        prepare(&mut view, &mut text);
    }
    assert_eq!(view.focused(), Some("four"));
    let scene = prepare(&mut view, &mut text);
    assert!(scene.scrolls[0].offset.y > 0.0);
    let r = scene.rects["four"];
    assert!(
        scene
            .hit(Point::new(r.origin.x + 5.0, r.origin.y + 5.0))
            .is_some()
    );
    let p = scene.scrolls[0].viewport.origin;
    view.handle_event(InputEvent::Wheel {
        position: Point::new(p.x + 1.0, p.y + 1.0),
        delta: Point::new(0.0, -10000.0),
    });
    let scene = prepare(&mut view, &mut text);
    assert_eq!(scene.scrolls[0].offset.y, 0.0);
    view.handle_event(InputEvent::Wheel {
        position: Point::new(p.x + 1.0, p.y + 1.0),
        delta: Point::new(0.0, f32::NAN),
    });
    let scene = prepare(&mut view, &mut text);
    assert_eq!(scene.scrolls[0].offset.y, 0.0);
}
#[test]
fn edits_keep_caret_visible_and_state_survives_equal_content_but_removed_keys_do_not() {
    let mut view = UiView::new("edit")
        .anchor(Anchor::TopLeft)
        .content(TextInput::new("name", "").width(80.0));
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    view.handle_event(InputEvent::Text("A long cluster name with spaces".into()));
    let scene = prepare(&mut view, &mut text);
    let c = &scene.controls[0];
    assert!(
        c.caret.origin.x >= c.content.origin.x
            && c.caret.origin.x < c.content.origin.x + c.content.size.width,
        "{c:?}"
    );
    view.set_content(TextInput::new("name", "").width(80.0));
    prepare(&mut view, &mut text);
    assert_eq!(
        view.text_value("name"),
        Some("A long cluster name with spaces")
    );
    view.set_content(Button::new("other", "Other"));
    prepare(&mut view, &mut text);
    assert_eq!(view.focused(), None);
    assert_eq!(view.text_value("name"), None);
}
#[test]
fn bidi_caret_and_hits_follow_shaped_positions_and_grapheme_boundaries() {
    let mut text = TextSystem::new();
    let value = "abc שלום 👩‍💻";
    let line = text.line_geometry(&Theme::default().font, value);
    for (index, x) in &line.stops {
        assert!(value.is_char_boundary(*index));
        assert!(x.is_finite());
    }
    let first_hebrew = value.find('ש').unwrap();
    let next = first_hebrew + 'ש'.len_utf8();
    assert!(line.x(first_hebrew) > line.x(next));
    assert!(
        line.hit(line.x(first_hebrew)) == first_hebrew
            || line.x(line.hit(line.x(first_hebrew))) == line.x(first_hebrew)
    );
}
#[test]
fn read_only_input_allows_selection_and_copy_but_rejects_edits() {
    let mut view =
        UiView::new("readonly").content(TextInput::new("name", "Locked").read_only(true));
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    key(&mut view, Key::A, false, true);
    assert_eq!(
        key(&mut view, Key::C, false, true),
        vec![UiEvent::ClipboardWrite("Locked".into())]
    );
    view.handle_event(InputEvent::Text("Wrong".into()));
    key(&mut view, Key::Backspace, false, false);
    assert_eq!(view.text_value("name"), Some("Locked"));
    assert!(matches!(view.ime_state(), UiEvent::Ime { key: None, .. }));
}

#[test]
fn surrounding_deletion_and_insertion_preserve_joined_grapheme_boundaries() {
    let mut view = UiView::new("edit").content(TextInput::new("name", "\u{200d}💻"));
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    key(&mut view, Key::Home, false, false);
    view.handle_event(InputEvent::Text("👩".into()));
    assert_eq!(view.text_value("name"), Some("👩‍💻"));
    assert_eq!(view.text_selection("name"), Some((11, 11)));
    view.handle_event(InputEvent::DeleteSurrounding {
        before: 1,
        after: 0,
    });
    assert_eq!(view.text_value("name"), Some(""));
}
#[test]
fn translated_ime_geometry_and_stale_structural_snapshots_are_safe() {
    let mut view = form();
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    key(&mut view, Key::Tab, false, false);
    let scene = prepare(&mut view, &mut text);
    let moved = scene.translated(Point::new(75.0, -20.0));
    let caret = match view.ime_state() {
        UiEvent::Ime { caret, .. } => caret,
        _ => panic!(),
    };
    let translated = match view.ime_state_in(&moved) {
        UiEvent::Ime { caret, .. } => caret,
        _ => panic!(),
    };
    assert_eq!(translated, caret.translated(Point::new(75.0, -20.0)));
    view.set_content(Button::new("other", "Other"));
    prepare(&mut view, &mut text);
    assert!(
        view.handle_event_in(
            &moved,
            InputEvent::PointerDown {
                position: moved.controls[0].rect.origin,
                shift: false
            }
        )
        .is_empty()
    );
}
#[test]
fn idle_pointer_movement_keeps_cached_layout_and_unknown_paste_does_not_consume_a_valid_reply() {
    let mut view = form();
    let mut text = TextSystem::new();
    prepare(&mut view, &mut text);
    assert!(!view.needs_prepare());
    view.handle_event(InputEvent::PointerMove {
        position: Point::new(-100.0, -100.0),
    });
    assert!(!view.needs_prepare());
    key(&mut view, Key::Tab, false, false);
    key(&mut view, Key::A, false, true);
    let request = match key(&mut view, Key::V, false, true)[0] {
        UiEvent::ClipboardRead { request } => request,
        _ => panic!(),
    };
    view.handle_event(InputEvent::Paste {
        request: request + 1,
        text: "Wrong".into(),
    });
    view.handle_event(InputEvent::Paste {
        request,
        text: "Correct".into(),
    });
    assert_eq!(view.text_value("name"), Some("Correct"));
}

#[test]
fn custom_button_content_keeps_full_row_activation_and_focus() {
    use halley_ui::input::{InputEvent, Key, Modifiers, UiEvent};
    use halley_ui::{ActionId, Button, Label, Rect, Row, TextSystem, Theme, UiView};
    let mut view = UiView::new("custom").content(
        Button::new("open", "")
            .accessible_label("Open editor")
            .action(ActionId::new("open-editor"))
            .width(300.0)
            .height(60.0)
            .child(Row::new("content").child(Label::new("title", "Editor"))),
    );
    let mut text = TextSystem::new();
    let prepared = view
        .prepare(
            Rect::new(0.0, 0.0, 300.0, 60.0),
            &Theme::default(),
            &mut text,
        )
        .unwrap();
    assert_eq!(prepared.controls.len(), 1);
    assert!(prepared.rects["title"].size.width > 0.0);
    view.handle_event(InputEvent::WindowFocus(true));
    view.handle_event(InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers::default(),
    });
    let events = view.handle_event(InputEvent::KeyDown {
        key: Key::Enter,
        modifiers: Modifiers::default(),
    });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, UiEvent::Activate(a) if a.0 == "open-editor"))
    );
}
