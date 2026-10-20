use halley_ui::ui::{PaintItem, PreparedView, SemanticRole};
use halley_ui::{
    ActionId, Anchor, Button, CloseReason, Column, Dialog, InputEvent, Key, Label, LayerEvent,
    LayerId, Menu, MenuItem, Modifiers, OverlayAnchor, OverlayError, Placement, Point, Popover,
    Rect, Size, TextInput, Theme, UiEvent, UiStack, UiView,
};

fn stack() -> UiStack {
    UiStack::new(
        UiView::new("app").anchor(Anchor::TopLeft).content(
            Column::new("page")
                .child(Button::new("open", "Actions"))
                .child(TextInput::new("name", "Original")),
        ),
    )
}
fn prepare_in(stack: &mut UiStack, bounds: Rect) -> PreparedView {
    stack
        .prepare_with_measure(bounds, &Theme::default(), |_, text| {
            Size::new(text.len() as f32 * 8.0, 16.0)
        })
        .unwrap()
        .clone()
}
fn prepare(stack: &mut UiStack) -> PreparedView {
    prepare_in(stack, Rect::new(0.0, 0.0, 600.0, 400.0))
}
fn key(stack: &mut UiStack, key: Key) -> Vec<LayerEvent> {
    stack.handle_event(InputEvent::KeyDown {
        key,
        modifiers: Modifiers::default(),
    })
}
fn center(rect: Rect) -> Point {
    Point::new(
        rect.origin.x + rect.size.width * 0.5,
        rect.origin.y + rect.size.height * 0.5,
    )
}
fn click(stack: &mut UiStack, point: Point) -> Vec<LayerEvent> {
    let mut events = stack.handle_event(InputEvent::PointerDown {
        position: point,
        shift: false,
    });
    events.extend(stack.handle_event(InputEvent::PointerUp { position: point }));
    events
}
fn menu() -> Menu {
    Menu::new("actions", OverlayAnchor::base("open"))
        .child(MenuItem::new("disabled", "Unavailable").disabled(true))
        .child(MenuItem::new("rename", "Rename"))
        .child(MenuItem::new("delete", "Delete").action(ActionId::new("confirm-delete")))
}
fn view_event(layer: LayerId, event: UiEvent) -> LayerEvent {
    LayerEvent::View { layer, event }
}
fn closed(key: &str, reason: CloseReason) -> LayerEvent {
    LayerEvent::Closed {
        key: key.into(),
        reason,
    }
}

#[test]
fn button_menu_confirmation_journey_restores_opener_and_scopes_actions() {
    let mut stack = stack();
    let base = prepare(&mut stack);
    assert!(
        click(&mut stack, center(base.rects["base/open"])).contains(&view_event(
            LayerId::Base,
            UiEvent::Activate(ActionId::new("open"))
        ))
    );
    stack.open(menu()).unwrap();
    let scene = prepare(&mut stack);
    let menu_id = LayerId::overlay("actions");
    assert_eq!(
        stack.view(&menu_id).unwrap().selected("actions"),
        Some("rename")
    );
    assert!(scene.controls.iter().all(|c| c.key.starts_with("overlay/")));
    key(&mut stack, Key::Down);
    assert_eq!(
        key(&mut stack, Key::Enter),
        vec![
            closed("actions", CloseReason::Activated),
            view_event(menu_id, UiEvent::Activate(ActionId::new("confirm-delete"))),
        ]
    );
    // The host can open a confirmation immediately while handling the action.
    stack
        .open(
            Dialog::new("confirm")
                .accessible_label("Delete item?")
                .initial_focus("cancel")
                .child(Label::new("question", "Delete this item?"))
                .child(Button::new("cancel", "Cancel"))
                .child(Button::new("delete", "Delete")),
        )
        .unwrap();
    let scene = prepare(&mut stack);
    let dialog = LayerId::overlay("confirm");
    assert_eq!(stack.view(&dialog).unwrap().focused(), Some("cancel"));
    assert!(
        scene
            .semantics
            .iter()
            .any(|s| matches!(s.role, SemanticRole::Dialog) && s.modal)
    );
    assert!(matches!(
        stack.focus(&LayerId::Base, "name"),
        Err(OverlayError::InactiveLayer(_))
    ));
    key(&mut stack, Key::Tab);
    assert_eq!(stack.view(&dialog).unwrap().focused(), Some("delete"));
    key(&mut stack, Key::Tab);
    assert_eq!(stack.view(&dialog).unwrap().focused(), Some("cancel"));
    stack.handle_event(InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers {
            shift: true,
            ..Default::default()
        },
    });
    assert_eq!(stack.view(&dialog).unwrap().focused(), Some("delete"));
    assert_eq!(
        key(&mut stack, Key::Escape),
        vec![closed("confirm", CloseReason::Escape)]
    );
    prepare(&mut stack);
    assert_eq!(stack.base().focused(), Some("open"));
}

#[test]
fn outside_dismissal_consumes_press_and_release_and_modal_blocks_background() {
    let mut stack = stack();
    let base = prepare(&mut stack);
    let opener = center(base.rects["base/open"]);
    stack.open(menu()).unwrap();
    prepare(&mut stack);
    assert_eq!(
        stack.handle_event(InputEvent::PointerDown {
            position: opener,
            shift: false
        }),
        vec![closed("actions", CloseReason::OutsidePress)]
    );
    prepare(&mut stack);
    assert!(
        stack
            .handle_event(InputEvent::PointerUp { position: opener })
            .is_empty()
    );
    assert!(click(&mut stack, opener).iter().any(|e| matches!(
        e,
        LayerEvent::View {
            event: UiEvent::Activate(_),
            ..
        }
    )));
    stack
        .open(Dialog::new("modal").child(TextInput::new("name", "Modal")))
        .unwrap();
    prepare(&mut stack);
    assert!(click(&mut stack, opener).is_empty());
    stack.handle_event(InputEvent::Text("!".into()));
    assert_eq!(stack.base().text_value("name"), Some("Original"));
    assert_eq!(
        stack
            .view(&LayerId::overlay("modal"))
            .unwrap()
            .text_value("name"),
        Some("Modal!")
    );
    assert!(stack.is_open("modal"));
}

#[test]
fn menu_navigation_wraps_skips_disabled_and_space_requires_unchanged_selection() {
    let mut stack = stack();
    prepare(&mut stack);
    stack.open(menu()).unwrap();
    prepare(&mut stack);
    let id = LayerId::overlay("actions");
    key(&mut stack, Key::Up);
    assert_eq!(stack.view(&id).unwrap().selected("actions"), Some("delete"));
    key(&mut stack, Key::Down);
    assert_eq!(stack.view(&id).unwrap().selected("actions"), Some("rename"));
    key(&mut stack, Key::Space);
    key(&mut stack, Key::Down);
    assert!(
        stack
            .handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    key(&mut stack, Key::Home);
    key(&mut stack, Key::Space);
    key(&mut stack, Key::Space);
    assert_eq!(
        stack.handle_event(InputEvent::KeyUp { key: Key::Space }),
        vec![
            closed("actions", CloseReason::Activated),
            view_event(id, UiEvent::Activate(ActionId::new("rename"))),
        ]
    );
}

#[test]
fn menu_hover_selects_and_click_activates_without_intervening_redraw() {
    let mut stack = stack();
    prepare(&mut stack);
    stack.open(menu()).unwrap();
    prepare(&mut stack);
    stack.take_events();
    let id = LayerId::overlay("actions");
    let point = center(stack.snapshot(&id).unwrap().rects["delete"]);
    let events = stack.handle_event(InputEvent::PointerMove { position: point });
    assert!(events.contains(&view_event(
        id.clone(),
        UiEvent::SelectionChanged {
            key: "actions".into(),
            selected: Some("delete".into()),
        }
    )));
    assert!(click(&mut stack, point).contains(&view_event(
        id,
        UiEvent::Activate(ActionId::new("confirm-delete"))
    )));
}

#[test]
fn overlays_flip_clamp_resize_and_escape_parent_clipping() {
    let mut stack = stack();
    let bounds = Rect::new(-100.0, -50.0, 300.0, 220.0);
    prepare_in(&mut stack, bounds);
    stack
        .open(
            Popover::new(
                "popup",
                OverlayAnchor::rect(Rect::new(175.0, 140.0, 20.0, 20.0)),
            )
            .width(120.0)
            .height(80.0)
            .padding(0.0),
        )
        .unwrap();
    prepare_in(&mut stack, bounds);
    let popup = stack.snapshot(&LayerId::overlay("popup")).unwrap();
    assert_eq!(stack.placement("popup"), Some(Placement::AboveStart));
    assert_eq!(popup.bounds, Rect::new(72.0, 56.0, 120.0, 80.0));
    assert!(popup.items.iter().all(|p| match p {
        PaintItem::Card { rect, clip, .. } => rect.intersection(*clip) == *rect,
        _ => true,
    }));
    let smaller = Rect::new(0.0, 0.0, 60.0, 40.0);
    prepare_in(&mut stack, smaller);
    let popup = stack.snapshot(&LayerId::overlay("popup")).unwrap();
    assert!(popup.bounds.size.width <= 44.0 && popup.bounds.size.height <= 24.0);
    assert_eq!(popup.bounds.intersection(smaller), popup.bounds);
    prepare_in(&mut stack, Rect::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn translated_frames_route_hits_and_ime_and_stale_frames_are_rejected() {
    let mut stack = stack();
    prepare(&mut stack);
    stack
        .open(
            Dialog::new("edit")
                .child(TextInput::new("name", "Edit"))
                .child(Button::new("save", "Save")),
        )
        .unwrap();
    let scene = prepare(&mut stack);
    let offset = Point::new(80.0, 25.0);
    let presented = scene.translated(offset);
    let id = LayerId::overlay("edit");
    let snapshot = stack.snapshot(&id).unwrap();
    let point = center(snapshot.rects["save"].translated(offset));
    let native_ime = stack.ime_state().unwrap();
    let translated_ime = stack.ime_state_in(&presented).unwrap();
    match (native_ime, translated_ime) {
        (
            LayerEvent::View {
                event: UiEvent::Ime { caret: a, .. },
                ..
            },
            LayerEvent::View {
                event: UiEvent::Ime { caret: b, .. },
                ..
            },
        ) => assert_eq!(a.translated(offset), b),
        other => panic!("missing editor IME state: {other:?}"),
    }
    stack.handle_event_in(
        &presented,
        InputEvent::PointerDown {
            position: point,
            shift: false,
        },
    );
    assert!(
        stack
            .handle_event_in(&presented, InputEvent::PointerUp { position: point })
            .contains(&view_event(id, UiEvent::Activate(ActionId::new("save"))))
    );
    stack.close_top();
    assert!(
        stack
            .handle_event_in(&presented, InputEvent::Text("Stale".into()))
            .is_empty()
    );
    prepare(&mut stack);
    assert!(
        stack
            .handle_event_in(
                &presented,
                InputEvent::PointerDown {
                    position: point,
                    shift: false
                }
            )
            .is_empty()
    );
}

#[test]
fn nested_layers_restore_focus_close_in_order_and_follow_anchor_removal() {
    let mut stack = stack();
    prepare(&mut stack);
    stack.focus(&LayerId::Base, "open").unwrap();
    stack
        .open(
            Popover::new("parent", OverlayAnchor::base("open"))
                .child(Button::new("open", "Nested")),
        )
        .unwrap();
    prepare(&mut stack);
    stack
        .open(
            Popover::new("child", OverlayAnchor::overlay("parent", "open"))
                .child(Button::new("open", "Leaf")),
        )
        .unwrap();
    prepare(&mut stack);
    assert_eq!(
        stack.close("parent").unwrap(),
        vec![
            closed("child", CloseReason::Programmatic),
            closed("parent", CloseReason::Programmatic),
        ]
    );
    prepare(&mut stack);
    assert_eq!(stack.base().focused(), Some("open"));
    stack.open(menu()).unwrap();
    prepare(&mut stack);
    stack.take_events();
    stack
        .base_mut()
        .set_content(Label::new("replacement", "No anchor"));
    prepare(&mut stack);
    assert_eq!(stack.active_layer(), LayerId::Base);
    assert!(
        stack
            .take_events()
            .contains(&closed("actions", CloseReason::AnchorLost))
    );
    assert_eq!(stack.base().focused(), None);
}

#[test]
fn definition_updates_preserve_editor_state_and_cancel_stale_space_activation() {
    let mut stack = stack();
    prepare(&mut stack);
    stack
        .open(Dialog::new("edit").child(TextInput::new("name", "Initial")))
        .unwrap();
    prepare(&mut stack);
    stack.handle_event(InputEvent::Text("!".into()));
    stack
        .update(
            Dialog::new("edit")
                .width(420.0)
                .child(TextInput::new("name", "Replacement seed")),
        )
        .unwrap();
    prepare(&mut stack);
    assert_eq!(
        stack
            .view(&LayerId::overlay("edit"))
            .unwrap()
            .text_value("name"),
        Some("Initial!")
    );
    stack.close_top();
    prepare(&mut stack);
    stack.open(menu()).unwrap();
    prepare(&mut stack);
    key(&mut stack, Key::Space);
    stack
        .update(
            Menu::new("actions", OverlayAnchor::base("open"))
                .child(MenuItem::new("rename", "Different action").action(ActionId::new("wrong"))),
        )
        .unwrap();
    prepare(&mut stack);
    assert!(
        stack
            .handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    assert!(stack.is_open("actions"));
}

#[test]
fn blur_closes_ephemeral_layers_composition_consumes_first_escape_and_dialog_persists() {
    let mut stack = stack();
    prepare(&mut stack);
    stack
        .open(Dialog::new("edit").child(TextInput::new("name", "")))
        .unwrap();
    prepare(&mut stack);
    stack.handle_event(InputEvent::Preedit {
        text: "日本".into(),
        cursor: None,
    });
    assert!(
        !key(&mut stack, Key::Escape)
            .iter()
            .any(|e| matches!(e, LayerEvent::Closed { .. }))
    );
    assert!(stack.is_open("edit"));
    stack
        .open(
            Menu::new("actions", OverlayAnchor::overlay("edit", "name"))
                .child(MenuItem::new("item", "Item")),
        )
        .unwrap();
    prepare(&mut stack);
    assert_eq!(
        stack.handle_event(InputEvent::WindowFocus(false)),
        vec![closed("actions", CloseReason::WindowBlur)]
    );
    assert!(stack.is_open("edit"));
    prepare(&mut stack);
    assert!(
        stack
            .handle_event(InputEvent::Text("Ignored".into()))
            .is_empty()
    );
    stack.handle_event(InputEvent::WindowFocus(true));
    prepare(&mut stack);
    assert_eq!(
        key(&mut stack, Key::Escape),
        vec![closed("edit", CloseReason::Escape)]
    );
}

#[test]
fn invalid_definitions_and_unprepared_anchors_fail_without_changing_layers() {
    let mut stack = stack();
    assert!(matches!(stack.open(menu()), Err(OverlayError::NotPrepared)));
    prepare(&mut stack);
    assert!(matches!(
        stack.open(Dialog::new("bad").width(f32::NAN)),
        Err(OverlayError::InvalidGeometry)
    ));
    assert!(matches!(
        stack.open(Popover::new("bad", OverlayAnchor::base("missing"))),
        Err(OverlayError::UnknownAnchor { .. })
    ));
    stack.open(menu()).unwrap();
    assert!(matches!(
        stack.open(menu()),
        Err(OverlayError::DuplicateKey(_))
    ));
    prepare(&mut stack);
    assert!(matches!(
        stack.update(Popover::new(
            "actions",
            OverlayAnchor::overlay("actions", "rename")
        )),
        Err(OverlayError::UnknownAnchor { .. })
    ));
    assert_eq!(stack.active_layer(), LayerId::overlay("actions"));
}

#[test]
fn placement_alignments_and_side_flips_use_window_coordinates() {
    let mut stack = stack();
    prepare(&mut stack);
    let anchor = Rect::new(250.0, 180.0, 40.0, 30.0);
    for (placement, expected) in [
        (Placement::BelowStart, Point::new(250.0, 214.0)),
        (Placement::BelowEnd, Point::new(190.0, 214.0)),
        (Placement::AboveStart, Point::new(250.0, 116.0)),
        (Placement::AboveEnd, Point::new(190.0, 116.0)),
        (Placement::RightStart, Point::new(294.0, 180.0)),
        (Placement::RightEnd, Point::new(294.0, 150.0)),
        (Placement::LeftStart, Point::new(146.0, 180.0)),
        (Placement::LeftEnd, Point::new(146.0, 150.0)),
        (Placement::Center, Point::new(250.0, 170.0)),
    ] {
        stack
            .open(
                Popover::new("align", OverlayAnchor::rect(anchor))
                    .placement(placement)
                    .width(100.0)
                    .height(60.0)
                    .padding(0.0),
            )
            .unwrap();
        prepare(&mut stack);
        assert_eq!(
            stack
                .snapshot(&LayerId::overlay("align"))
                .unwrap()
                .bounds
                .origin,
            expected
        );
        stack.close_top();
        prepare(&mut stack);
    }
    for (anchor, placement, flipped) in [
        (
            Rect::new(580.0, 390.0, 10.0, 5.0),
            Placement::RightStart,
            Placement::LeftStart,
        ),
        (
            Rect::new(8.0, 8.0, 10.0, 5.0),
            Placement::LeftEnd,
            Placement::RightEnd,
        ),
        (
            Rect::new(8.0, 8.0, 10.0, 5.0),
            Placement::AboveEnd,
            Placement::BelowEnd,
        ),
    ] {
        stack
            .open(
                Popover::new("flip", OverlayAnchor::rect(anchor))
                    .placement(placement)
                    .width(100.0)
                    .height(60.0),
            )
            .unwrap();
        prepare(&mut stack);
        assert_eq!(stack.placement("flip"), Some(flipped));
        stack.close_top();
        prepare(&mut stack);
    }
}

#[test]
fn oversized_menu_and_dialog_scroll_and_reveal_inside_small_windows() {
    let mut stack = stack();
    let bounds = Rect::new(0.0, 0.0, 240.0, 160.0);
    prepare_in(&mut stack, bounds);
    let mut menu = Menu::new("long", OverlayAnchor::base("open"));
    for i in 0..30 {
        menu = menu.child(MenuItem::new(format!("item-{i}"), format!("Item {i}")).height(30.0));
    }
    stack.open(menu).unwrap();
    prepare_in(&mut stack, bounds);
    key(&mut stack, Key::End);
    prepare_in(&mut stack, bounds);
    let source = stack.snapshot(&LayerId::overlay("long")).unwrap();
    let item = source.controls.iter().find(|c| c.key == "item-29").unwrap();
    assert_eq!(item.rect.intersection(item.clip), item.rect);
    assert!(source.scrolls[0].offset.y > 0.0);
    stack.close_top();
    prepare_in(&mut stack, bounds);
    let mut dialog = Dialog::new("long").initial_focus("button-29");
    for i in 0..30 {
        dialog = dialog.child(Button::new(format!("button-{i}"), "Button").height(30.0));
    }
    stack.open(dialog).unwrap();
    prepare_in(&mut stack, bounds);
    let source = stack.snapshot(&LayerId::overlay("long")).unwrap();
    let button = source
        .controls
        .iter()
        .find(|c| c.key == "button-29")
        .unwrap();
    assert_eq!(button.rect.intersection(button.clip), button.rect);
    assert!(source.scrolls[0].offset.y > 0.0);
    assert_eq!(source.bounds.intersection(bounds), source.bounds);
    for size in [0.0, 1.0, 12.0, 30.0] {
        let tiny = Rect::new(0.0, 0.0, size, size);
        prepare_in(&mut stack, tiny);
        let source = stack.snapshot(&LayerId::overlay("long")).unwrap();
        assert_eq!(source.bounds.intersection(tiny), source.bounds);
    }
}

fn paste_request(stack: &mut UiStack) -> u64 {
    stack
        .handle_event(InputEvent::KeyDown {
            key: Key::V,
            modifiers: Modifiers {
                control: true,
                ..Default::default()
            },
        })
        .into_iter()
        .find_map(|e| match e {
            LayerEvent::View {
                event: UiEvent::ClipboardRead { request },
                ..
            } => Some(request),
            _ => None,
        })
        .unwrap()
}

#[test]
fn delayed_clipboard_replies_cannot_cross_layers_or_reopened_overlay_instances() {
    let mut stack = stack();
    prepare(&mut stack);
    stack.focus(&LayerId::Base, "name").unwrap();
    let base_request = paste_request(&mut stack);
    let dialog = || Dialog::new("edit").child(TextInput::new("name", ""));
    let id = LayerId::overlay("edit");
    stack.open(dialog()).unwrap();
    prepare(&mut stack);
    let old_request = paste_request(&mut stack);
    assert!(
        stack
            .handle_event_for(
                &LayerId::Base,
                InputEvent::Paste {
                    request: base_request,
                    text: "Wrong layer".into()
                }
            )
            .is_empty()
    );
    stack.close_top();
    prepare(&mut stack);
    stack.open(dialog()).unwrap();
    prepare(&mut stack);
    let new_request = paste_request(&mut stack);
    assert_ne!(old_request, new_request);
    stack.handle_event_for(
        &id,
        InputEvent::Paste {
            request: old_request,
            text: "Stale instance".into(),
        },
    );
    stack.handle_event_for(
        &id,
        InputEvent::Paste {
            request: new_request,
            text: "Correct".into(),
        },
    );
    assert_eq!(stack.view(&id).unwrap().text_value("name"), Some("Correct"));
    assert_eq!(stack.base().text_value("name"), Some("Original"));
}

#[test]
fn a_dismissed_presented_frame_still_consumes_its_release_before_the_next_frame() {
    let mut stack = stack();
    let scene = prepare(&mut stack);
    let point = center(scene.rects["base/open"]);
    stack.open(menu()).unwrap();
    let scene = prepare(&mut stack);
    stack.handle_event_in(
        &scene,
        InputEvent::PointerDown {
            position: point,
            shift: false,
        },
    );
    assert!(
        stack
            .handle_event_in(&scene, InputEvent::PointerUp { position: point })
            .is_empty()
    );
    prepare(&mut stack);
    assert!(click(&mut stack, point).contains(&view_event(
        LayerId::Base,
        UiEvent::Activate(ActionId::new("open"))
    )));
}

#[test]
fn replacement_actions_cannot_inherit_pointer_capture_and_explicit_focus_wins_restoration() {
    let mut stack = stack();
    prepare(&mut stack);
    stack.focus(&LayerId::Base, "open").unwrap();
    stack.open(menu()).unwrap();
    prepare(&mut stack);
    let point = center(stack.snapshot(&LayerId::overlay("actions")).unwrap().rects["rename"]);
    stack.handle_event(InputEvent::PointerDown {
        position: point,
        shift: false,
    });
    stack
        .update(
            Menu::new("actions", OverlayAnchor::base("open"))
                .child(MenuItem::new("disabled", "Unavailable").disabled(true))
                .child(
                    MenuItem::new("rename", "Replacement action").action(ActionId::new("wrong")),
                ),
        )
        .unwrap();
    prepare(&mut stack);
    assert!(
        stack
            .handle_event(InputEvent::PointerUp { position: point })
            .is_empty()
    );
    stack.close_top();
    stack.focus(&LayerId::Base, "name").unwrap();
    prepare(&mut stack);
    assert_eq!(stack.base().focused(), Some("name"));
}

#[test]
fn dismissal_policies_empty_menus_disabled_openers_and_invalid_geometry_are_safe() {
    let mut stack = stack();
    prepare(&mut stack);
    stack.focus(&LayerId::Base, "open").unwrap();
    stack
        .open(Dialog::new("locked").dismiss_on_escape(false))
        .unwrap();
    prepare(&mut stack);
    assert!(key(&mut stack, Key::Escape).is_empty());
    stack
        .base_mut()
        .set_content(Button::new("open", "Disabled opener").disabled(true));
    prepare(&mut stack);
    stack.close_top();
    prepare(&mut stack);
    assert_eq!(stack.base().focused(), None);
    stack
        .open(Menu::new(
            "empty",
            OverlayAnchor::rect(Rect::new(0.0, 0.0, 0.0, 0.0)),
        ))
        .unwrap();
    prepare(&mut stack);
    assert!(key(&mut stack, Key::Down).is_empty());
    assert!(key(&mut stack, Key::Enter).is_empty());
    assert!(stack.is_open("empty"));
    stack
        .update(
            Menu::new("empty", OverlayAnchor::rect(Rect::new(0.0, 0.0, 0.0, 0.0)))
                .child(MenuItem::new("disabled", "Disabled").disabled(true)),
        )
        .unwrap();
    prepare(&mut stack);
    assert!(key(&mut stack, Key::Enter).is_empty());
    assert!(
        stack
            .prepare_with_measure(
                Rect::new(0.0, 0.0, f32::NAN, 100.0),
                &Theme::default(),
                |_, _| Size::default()
            )
            .is_err()
    );
    assert!(stack.needs_prepare());
    assert!(key(&mut stack, Key::Escape).is_empty());
    prepare(&mut stack);
    assert_eq!(
        key(&mut stack, Key::Escape),
        vec![closed("empty", CloseReason::Escape)]
    );
}

#[test]
fn failed_preparation_can_recover_to_the_previous_valid_theme_and_geometry() {
    let mut stack = stack();
    let scene = prepare(&mut stack);
    let mut invalid = Theme::default();
    invalid.overlays.backdrop.a = f32::NAN;
    assert!(
        stack
            .prepare_with_measure(scene.bounds, &invalid, |_, _| Size::default())
            .is_err()
    );
    assert!(stack.needs_prepare());
    let scene = prepare(&mut stack);
    assert!(
        click(&mut stack, center(scene.rects["base/open"])).contains(&view_event(
            LayerId::Base,
            UiEvent::Activate(ActionId::new("open"))
        ))
    );
    stack.open(menu()).unwrap();
}

#[test]
fn theme_changes_repaint_layers_and_backdrop_composes_above_the_base() {
    use halley_ui::software::PixelBuffer;
    use halley_ui::{Card, CardStyle, Color, TextSystem};
    let chrome = |color| CardStyle {
        fill: color,
        border: color,
        border_width: 0.0,
        radius: 0.0,
    };
    let red = Color::rgb8(200, 0, 0);
    let green = Color::rgb8(0, 200, 0);
    let blue = Color::rgb8(0, 0, 200);
    let mut stack = UiStack::new(
        UiView::new("pixels").content(
            Card::new("base")
                .width(200.0)
                .height(120.0)
                .style(chrome(red)),
        ),
    );
    let bounds = Rect::new(0.0, 0.0, 200.0, 120.0);
    let mut theme = Theme::default();
    theme.overlays.dialog = Some(chrome(green));
    theme.overlays.backdrop = Color::rgba(0.0, 0.0, 0.0, 0.5);
    let mut text = TextSystem::new();
    stack.prepare(bounds, &theme, &mut text).unwrap();
    stack
        .open(Dialog::new("color").width(60.0).height(40.0))
        .unwrap();
    let mut pixels = PixelBuffer::new(200, 120).unwrap();
    pixels.surface().draw(
        stack.prepare(bounds, &theme, &mut text).unwrap(),
        &mut text,
        1.0,
    );
    let pixel = |buffer: &PixelBuffer, x: usize, y: usize| {
        buffer.pixels()[(y * 200 + x) * 4..(y * 200 + x) * 4 + 4].to_vec()
    };
    assert_eq!(pixel(&pixels, 1, 1), vec![100, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 100, 60), vec![0, 200, 0, 255]);
    theme.overlays.dialog = Some(chrome(blue));
    pixels.surface().draw(
        stack.prepare(bounds, &theme, &mut text).unwrap(),
        &mut text,
        1.0,
    );
    assert_eq!(pixel(&pixels, 100, 60), vec![0, 0, 200, 255]);
}

#[cfg(feature = "accessibility")]
#[test]
fn accessible_menus_and_modal_dialogs_expose_only_the_active_scope() {
    struct Changes(usize);
    impl accesskit_consumer::TreeChangeHandler for Changes {
        fn node_added(&mut self, _: &accesskit_consumer::NodeRef) {}
        fn node_updated(
            &mut self,
            _: &accesskit_consumer::NodeRef,
            _: &accesskit_consumer::NodeRef,
        ) {
        }
        fn focus_moved(
            &mut self,
            _: Option<&accesskit_consumer::NodeRef>,
            _: Option<&accesskit_consumer::NodeRef>,
        ) {
        }
        fn node_removed(&mut self, _: &accesskit_consumer::NodeRef) {
            self.0 += 1;
        }
    }
    let mut changes = Changes(0);
    use halley_ui::accessibility::{
        AccessibilityTree, AccessibleAction,
        accesskit::{Action, ActionRequest, Role, TreeId},
    };
    let mut stack = stack();
    let scene = prepare(&mut stack);
    let mut tree = AccessibilityTree::default();
    let update = tree.update(&scene, "App");
    let opener = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Button)
        .unwrap()
        .0;
    let stale = tree
        .action(ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: opener,
            data: None,
        })
        .unwrap();
    let mut consumer = accesskit_consumer::Tree::new(update, true);
    stack.open(menu()).unwrap();
    let scene = prepare(&mut stack);
    assert!(stack.handle_accessibility(stale).is_empty());
    let update = tree.update(&scene, "App");
    assert!(update.nodes.iter().any(|(_, n)| n.role() == Role::Menu));
    let item = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::MenuItem && n.label() == Some("Delete"))
        .unwrap();
    let action = tree
        .action(ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: item.0,
            data: None,
        })
        .unwrap();
    consumer.update_and_process_changes(update, &mut changes);
    assert!(changes.0 > 0);
    assert!(
        stack
            .handle_accessibility(action)
            .contains(&closed("actions", CloseReason::Activated))
    );
    stack
        .open(
            Dialog::new("modal")
                .accessible_label("Confirmation")
                .child(Button::new("open", "Confirm")),
        )
        .unwrap();
    let scene = prepare(&mut stack);
    let update = tree.update(&scene, "App");
    let dialog = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Dialog)
        .unwrap();
    assert!(dialog.1.is_modal());
    assert_eq!(dialog.1.label(), Some("Confirmation"));
    assert!(
        !update
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Actions"))
    );
    consumer.update_and_process_changes(update, &mut changes);
    assert!(
        stack
            .handle_accessibility(AccessibleAction::Activate("base/open".into()))
            .is_empty()
    );
}
