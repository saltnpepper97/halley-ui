use halley_ui::{
    Anchor, Button, Checkbox, Column, InputEvent, InteractionError, Key, Label, List, ListItem,
    Modifiers, Point, RadioGroup, RadioOption, Rect, Scroll, Size, Switch, Theme, UiEvent, UiView,
    ui::{LayoutError, PaintItem, PreparedView, SemanticRole},
};

fn options(order: &[&str]) -> RadioGroup {
    let mut group = RadioGroup::new("mode")
        .selected("balanced")
        .accessible_label("Power mode");
    for key in order {
        group = group.child(RadioOption::new(*key, *key).disabled(*key == "unavailable"));
    }
    group
}
fn content() -> Column {
    Column::new("settings")
        .gap(8.0)
        .child(Checkbox::new("notifications", "Notifications").checked(true))
        .child(Switch::new("network", "Network"))
        .child(options(&[
            "performance",
            "balanced",
            "unavailable",
            "quiet",
        ]))
        .child(Button::new("done", "Done"))
}
fn view() -> UiView {
    UiView::new("settings")
        .anchor(Anchor::TopLeft)
        .content(content())
}
fn prepare_with(view: &mut UiView, theme: &Theme) -> PreparedView {
    view.prepare_with_measure(Rect::new(20.0, 30.0, 440.0, 430.0), theme, |font, value| {
        Size::new(
            value.chars().count() as f32 * font.size as f32 * 0.6,
            font.size as f32 * 1.2,
        )
    })
    .unwrap()
    .clone()
}
fn prepare(view: &mut UiView) -> PreparedView {
    prepare_with(view, &Theme::default())
}
fn key(view: &mut UiView, key: Key) -> Vec<UiEvent> {
    view.handle_event(InputEvent::KeyDown {
        key,
        modifiers: Modifiers::default(),
    })
}
fn space(view: &mut UiView) -> Vec<UiEvent> {
    key(view, Key::Space);
    view.handle_event(InputEvent::KeyUp { key: Key::Space })
}
fn center(rect: Rect) -> Point {
    Point::new(
        rect.origin.x + rect.size.width * 0.5,
        rect.origin.y + rect.size.height * 0.5,
    )
}
fn click(view: &mut UiView, point: Point) -> Vec<UiEvent> {
    let mut events = view.handle_event(InputEvent::PointerDown {
        position: point,
        shift: false,
    });
    events.extend(view.handle_event(InputEvent::PointerUp { position: point }));
    events
}

#[test]
fn pointer_and_keyboard_toggle_once_and_emit_values_instead_of_actions() {
    let mut view = view();
    let scene = prepare(&mut view);
    assert_eq!(view.checked("notifications"), Some(true));
    let label_point = Point::new(
        scene.rects["notifications"].origin.x + 70.0,
        center(scene.rects["notifications"]).y,
    );
    assert!(
        click(&mut view, label_point).contains(&UiEvent::CheckedChanged {
            key: "notifications".into(),
            checked: false
        })
    );
    view.focus("network").unwrap();
    for _ in 0..4 {
        assert!(key(&mut view, Key::Space).is_empty());
    }
    assert_eq!(view.checked("network"), Some(false));
    assert_eq!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space }),
        vec![UiEvent::CheckedChanged {
            key: "network".into(),
            checked: true
        }]
    );
    assert!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    assert!(key(&mut view, Key::Enter).is_empty());
    assert_eq!(view.checked("network"), Some(true));
    assert!(view.needs_prepare());
}

#[test]
fn radio_group_is_one_tab_stop_and_arrows_wrap_skip_disabled_and_select() {
    let mut view = view();
    prepare(&mut view);
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("notifications"));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("network"));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("balanced"));
    let events = key(&mut view, Key::Down);
    assert!(events.contains(&UiEvent::RadioChanged {
        key: "mode".into(),
        selected: Some("quiet".into())
    }));
    assert_eq!(view.focused(), Some("quiet"));
    key(&mut view, Key::Right);
    assert_eq!(view.radio_selected("mode"), Some("performance"));
    key(&mut view, Key::Left);
    assert_eq!(view.radio_selected("mode"), Some("quiet"));
    key(&mut view, Key::Home);
    assert_eq!(view.radio_selected("mode"), Some("performance"));
    key(&mut view, Key::End);
    assert_eq!(view.radio_selected("mode"), Some("quiet"));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("done"));
    view.handle_event(InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers {
            shift: true,
            ..Default::default()
        },
    });
    assert_eq!(view.focused(), Some("quiet"));
    assert!(space(&mut view).is_empty());
    // Programmatic focus can visit an unselected option without changing the value.
    view.focus("balanced").unwrap();
    assert_eq!(view.radio_selected("mode"), Some("quiet"));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("done"));
}

#[test]
fn seeds_apply_once_reordering_preserves_state_and_removal_resets_it() {
    let mut view = view();
    prepare(&mut view);
    view.set_checked("notifications", false).unwrap();
    view.set_radio_selected("mode", Some("quiet")).unwrap();
    view.set_content(
        Column::new("settings")
            .child(Checkbox::new("notifications", "New label").checked(true))
            .child(options(&["quiet", "balanced", "performance"])),
    );
    prepare(&mut view);
    assert_eq!(view.checked("notifications"), Some(false));
    assert_eq!(view.radio_selected("mode"), Some("quiet"));
    view.set_content(options(&["balanced", "performance"]));
    prepare(&mut view);
    assert_eq!(view.radio_selected("mode"), None);
    assert_eq!(view.checked("notifications"), None);
    view.set_content(Label::new("empty", "Empty"));
    prepare(&mut view);
    view.set_content(content());
    prepare(&mut view);
    assert_eq!(view.checked("notifications"), Some(true));
    assert_eq!(view.radio_selected("mode"), Some("balanced"));
}

#[test]
fn disabled_values_remain_meaningful_and_programmatic_updates_are_silent() {
    let mut view = UiView::new("disabled").content(
        Column::new("settings")
            .child(Checkbox::new("check", "Check").disabled(true))
            .child(Switch::new("switch", "Switch").disabled(true).checked(true))
            .child(
                RadioGroup::new("group")
                    .selected("one")
                    .disabled(true)
                    .child(RadioOption::new("one", "One"))
                    .child(RadioOption::new("two", "Two")),
            )
            .child(Button::new("done", "Done")),
    );
    let scene = prepare(&mut view);
    assert!(click(&mut view, center(scene.rects["check"])).is_empty());
    assert!(click(&mut view, center(scene.rects["two"])).is_empty());
    assert_eq!(view.radio_selected("group"), Some("one"));
    assert!(matches!(
        view.focus("one"),
        Err(InteractionError::Disabled(_))
    ));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("done"));
    assert!(view.set_checked("check", true).unwrap());
    assert!(!view.set_checked("check", true).unwrap());
    assert!(view.set_radio_selected("group", Some("two")).unwrap());
    assert!(!view.set_radio_selected("group", Some("two")).unwrap());
    let scene = prepare(&mut view);
    assert_eq!(
        scene
            .semantics
            .iter()
            .find(|s| s.key == "two")
            .unwrap()
            .checked,
        Some(true)
    );
    assert!(
        scene
            .controls
            .iter()
            .find(|c| c.key == "two")
            .unwrap()
            .disabled
    );
}

#[test]
fn no_selection_empty_groups_and_all_disabled_groups_have_sensible_focus() {
    let mut view = UiView::new("groups").content(
        Column::new("root")
            .child(Button::new("before", "Before"))
            .child(RadioGroup::new("empty"))
            .child(
                RadioGroup::new("locked")
                    .child(RadioOption::new("disabled", "Disabled").disabled(true)),
            )
            .child(
                RadioGroup::new("unselected")
                    .child(RadioOption::new("one", "One"))
                    .child(RadioOption::new("two", "Two")),
            )
            .child(Button::new("after", "After")),
    );
    prepare(&mut view);
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("before"));
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("one"));
    assert_eq!(view.radio_selected("unselected"), None);
    assert_eq!(
        space(&mut view),
        vec![UiEvent::RadioChanged {
            key: "unselected".into(),
            selected: Some("one".into())
        }]
    );
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("after"));
    assert!(view.focus("empty").is_err());
    view.focus("unselected").unwrap();
    assert_eq!(view.focused(), Some("one"));
}

#[test]
fn capture_cancels_on_focus_blur_escape_disable_and_programmatic_value_changes() {
    let mut view = view();
    let scene = prepare(&mut view);
    view.focus("network").unwrap();
    key(&mut view, Key::Space);
    view.focus("notifications").unwrap();
    assert!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    view.focus("network").unwrap();
    key(&mut view, Key::Space);
    key(&mut view, Key::Escape);
    assert!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    key(&mut view, Key::Space);
    view.handle_event(InputEvent::WindowFocus(false));
    view.handle_event(InputEvent::WindowFocus(true));
    assert!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    let point = center(scene.rects["network"]);
    view.handle_event(InputEvent::PointerDown {
        position: point,
        shift: false,
    });
    view.set_checked("network", true).unwrap();
    assert!(
        view.handle_event(InputEvent::PointerUp { position: point })
            .is_empty()
    );
    view.focus("network").unwrap();
    key(&mut view, Key::Space);
    view.set_content(Switch::new("network", "Disabled").disabled(true));
    prepare(&mut view);
    assert!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    assert_eq!(view.checked("network"), Some(true));
}

#[test]
fn translated_label_clicks_respect_clips_cancellation_and_structural_snapshot_revisions() {
    let mut view = view();
    let scene = prepare(&mut view).translated(Point::new(90.0, 55.0));
    let point = center(scene.rects["quiet"]);
    view.handle_event_in(
        &scene,
        InputEvent::PointerDown {
            position: point,
            shift: false,
        },
    );
    assert_eq!(
        view.handle_event_in(&scene, InputEvent::PointerUp { position: point }),
        vec![UiEvent::RadioChanged {
            key: "mode".into(),
            selected: Some("quiet".into())
        }]
    );
    view.handle_event_in(
        &scene,
        InputEvent::PointerDown {
            position: point,
            shift: false,
        },
    );
    view.handle_event(InputEvent::PointerCancel);
    assert!(
        view.handle_event_in(&scene, InputEvent::PointerUp { position: point })
            .is_empty()
    );
    view.set_content(Checkbox::new("quiet", "Replacement"));
    assert!(
        view.handle_event_in(
            &scene,
            InputEvent::PointerDown {
                position: point,
                shift: false
            }
        )
        .is_empty()
    );
    prepare(&mut view);
    assert_eq!(view.checked("quiet"), Some(false));
}

#[test]
fn radio_arrows_reveal_offscreen_choices_without_allowing_clipped_pointer_hits() {
    let mut group = RadioGroup::new("group");
    for i in 0..20 {
        group =
            group.child(RadioOption::new(format!("item-{i}"), format!("Item {i}")).height(30.0));
    }
    let mut view = UiView::new("scroll").content(
        Scroll::new("viewport")
            .width(220.0)
            .height(100.0)
            .child(group),
    );
    let scene = prepare(&mut view);
    let point = center(scene.rects["item-19"]);
    assert!(click(&mut view, point).is_empty());
    view.focus("group").unwrap();
    key(&mut view, Key::End);
    let scene = prepare(&mut view);
    let last = scene.controls.iter().find(|c| c.key == "item-19").unwrap();
    assert_eq!(last.rect.intersection(last.clip), last.rect);
    assert!(scene.scrolls[0].offset.y > 0.0);
}

#[test]
fn validation_rejects_radio_options_outside_groups_and_interactive_list_content() {
    let mut view = UiView::new("invalid").content(RadioOption::new("orphan", "Orphan"));
    assert!(matches!(
        view.prepare_with_measure(
            Rect::new(0.0, 0.0, 200.0, 100.0),
            &Theme::default(),
            |_, _| Size::default()
        ),
        Err(LayoutError::InvalidRadioOption(_))
    ));
    view.set_content(
        List::new("list")
            .child(ListItem::new("item", "Item").child(Checkbox::new("nested", "Nested"))),
    );
    assert!(matches!(
        view.prepare_with_measure(
            Rect::new(0.0, 0.0, 200.0, 100.0),
            &Theme::default(),
            |_, _| Size::default()
        ),
        Err(LayoutError::InvalidListItem(_))
    ));
    view.set_content(Checkbox::new("valid", "Valid"));
    prepare(&mut view);
    assert!(matches!(
        view.set_radio_selected("valid", None),
        Err(InteractionError::NotRadioGroup(_))
    ));
    assert!(matches!(
        view.set_checked("missing", true),
        Err(InteractionError::NotCheckable(_))
    ));
    view.set_content(options(&["balanced"]));
    prepare(&mut view);
    assert!(matches!(
        view.set_radio_selected("mode", Some("missing")),
        Err(InteractionError::InvalidRadioOption { .. })
    ));
}

#[test]
fn form_styles_and_metrics_drive_layout_and_paint_without_losing_values() {
    use halley_ui::{Color, Outline, Style, StyleSheet};
    let mut view = view();
    let first = prepare(&mut view);
    view.set_checked("network", true).unwrap();
    view.focus("network").unwrap();
    let mut theme = Theme::default();
    theme.font.size = 22;
    theme.forms.indicator_size = 24.0;
    theme.forms.switch_width = 48.0;
    theme.forms.indicator_gap = 13.0;
    let on = Color::rgb8(30, 160, 80);
    theme.styles.switch.selected.fill = Some(on);
    theme.styles.switch.focused.outline = Some(Outline {
        color: on,
        width: 3.0,
    });
    let scene = prepare_with(&mut view, &theme);
    assert!(scene.rects["network"].size.width > first.rects["network"].size.width);
    assert_eq!(view.checked("network"), Some(true));
    assert!(scene.items.iter().any(|item| matches!(item, PaintItem::Card { key, rect, style, .. } if key == "settings/network/indicator" && style.fill == on && rect.size.width == 48.0)));
    assert!(scene.items.iter().any(|item| matches!(item, PaintItem::Card { key, style, .. } if key == "settings/network/focus" && style.border_width == 3.0)));
    let shared = Color::rgb8(170, 90, 20);
    let local = Color::rgb8(40, 100, 180);
    view.set_content(
        RadioGroup::new("mode")
            .selected("balanced")
            .option_style(StyleSheet {
                selected: Style {
                    fill: Some(shared),
                    ..Default::default()
                },
                ..Default::default()
            })
            .child(RadioOption::new("balanced", "Balanced").style(StyleSheet {
                selected: Style {
                    fill: Some(local),
                    ..Default::default()
                },
                ..Default::default()
            })),
    );
    let scene = prepare_with(&mut view, &theme);
    assert!(scene.items.iter().any(|item| matches!(item, PaintItem::Card { key, style, .. } if key == "settings/balanced/indicator" && style.fill == local)));
    assert!(
        scene
            .semantics
            .iter()
            .any(|s| matches!(s.role, SemanticRole::RadioOption) && s.checked == Some(true))
    );
}

#[test]
fn invalid_metrics_and_inactive_component_rules_are_rejected() {
    let mut view = view();
    let mut theme = Theme::default();
    theme.forms.switch_width = 1.0;
    assert!(
        view.prepare_with_measure(Rect::new(0.0, 0.0, 400.0, 300.0), &theme, |_, _| {
            Size::default()
        })
        .is_err()
    );
    theme = Theme::default();
    theme.styles.checkbox.disabled.radius = Some(f32::NAN);
    assert!(
        view.prepare_with_measure(Rect::new(0.0, 0.0, 400.0, 300.0), &theme, |_, _| {
            Size::default()
        })
        .is_err()
    );
}

#[cfg(feature = "accessibility")]
#[test]
fn accessibility_exposes_checked_state_roles_and_uses_the_same_change_events() {
    use halley_ui::accessibility::{
        AccessibilityTree, AccessibleAction,
        accesskit::{Action, ActionRequest, Role, Toggled, TreeId},
    };
    let mut view = view();
    let scene = prepare(&mut view);
    let mut tree = AccessibilityTree::default();
    let update = tree.update(&scene, "Settings");
    let _ = accesskit_consumer::Tree::new(update.clone(), true);
    let checkbox = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::CheckBox)
        .unwrap();
    assert_eq!(checkbox.1.label(), Some("Notifications"));
    assert_eq!(checkbox.1.toggled(), Some(Toggled::True));
    let switch = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Switch)
        .unwrap();
    assert_eq!(switch.1.toggled(), Some(Toggled::False));
    let radio = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::RadioButton && n.label() == Some("quiet"))
        .unwrap();
    let action = tree
        .action(ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: radio.0,
            data: None,
        })
        .unwrap();
    assert_eq!(
        view.handle_accessibility(action),
        vec![UiEvent::RadioChanged {
            key: "mode".into(),
            selected: Some("quiet".into())
        }]
    );
    assert_eq!(
        view.handle_accessibility(AccessibleAction::Activate("notifications".into())),
        vec![UiEvent::CheckedChanged {
            key: "notifications".into(),
            checked: false
        }]
    );
    assert!(
        view.handle_accessibility(AccessibleAction::Activate("unavailable".into()))
            .is_empty()
    );
    view.focus("notifications").unwrap();
    key(&mut view, Key::Space);
    assert_eq!(
        view.handle_accessibility(AccessibleAction::Activate("notifications".into())),
        vec![UiEvent::CheckedChanged {
            key: "notifications".into(),
            checked: true
        }]
    );
    assert!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    let scene = prepare(&mut view);
    let update = tree.update(&scene, "Settings");
    let _ = accesskit_consumer::Tree::new(update.clone(), true);
    assert_eq!(
        update
            .nodes
            .iter()
            .find(|(_, n)| n.role() == Role::RadioButton && n.label() == Some("quiet"))
            .unwrap()
            .1
            .toggled(),
        Some(Toggled::True)
    );
}

#[test]
fn settings_in_a_dialog_keep_their_layer_events_and_do_not_trigger_auto_close() {
    use halley_ui::ui::ControlKind;
    use halley_ui::{Dialog, LayerEvent, LayerId, UiStack};
    let mut stack = UiStack::new(UiView::new("base").content(Button::new("open", "Open")));
    let bounds = Rect::new(0.0, 0.0, 500.0, 450.0);
    let prepare = |stack: &mut UiStack| {
        stack
            .prepare_with_measure(bounds, &Theme::default(), |_, text| {
                Size::new(text.len() as f32 * 7.0, 16.0)
            })
            .unwrap()
            .clone()
    };
    prepare(&mut stack);
    stack
        .open(Dialog::new("form").close_on_activate(true).child(content()))
        .unwrap();
    let scene = prepare(&mut stack);
    let id = LayerId::overlay("form");
    let option = scene
        .controls
        .iter()
        .find(|c| c.key.ends_with("/balanced"))
        .unwrap();
    assert!(
        matches!(&option.kind, ControlKind::RadioOption { group } if group == "overlay/4:form/mode")
    );
    stack.handle_event(InputEvent::KeyDown {
        key: Key::Space,
        modifiers: Modifiers::default(),
    });
    assert_eq!(
        stack.handle_event(InputEvent::KeyUp { key: Key::Space }),
        vec![LayerEvent::View {
            layer: id.clone(),
            event: UiEvent::CheckedChanged {
                key: "notifications".into(),
                checked: false
            }
        }]
    );
    assert!(stack.is_open("form"));
    stack.handle_event(InputEvent::KeyDown {
        key: Key::Space,
        modifiers: Modifiers::default(),
    });
    stack.update(Dialog::new("form").child(content())).unwrap();
    prepare(&mut stack);
    assert!(
        stack
            .handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    assert_eq!(
        stack.view(&id).unwrap().checked("notifications"),
        Some(false)
    );
    stack.set_checked(&id, "notifications", true).unwrap();
    stack
        .set_radio_selected(&id, "mode", Some("quiet"))
        .unwrap();
    prepare(&mut stack);
    assert_eq!(
        stack.view(&id).unwrap().checked("notifications"),
        Some(true)
    );
    assert_eq!(
        stack.view(&id).unwrap().radio_selected("mode"),
        Some("quiet")
    );
    stack
        .open(Dialog::new("nested").child(Button::new("close", "Close")))
        .unwrap();
    prepare(&mut stack);
    stack.set_checked(&id, "notifications", false).unwrap();
    prepare(&mut stack);
    assert_eq!(
        stack.view(&id).unwrap().checked("notifications"),
        Some(false)
    );
    assert_eq!(stack.active_layer(), LayerId::overlay("nested"));
}

#[test]
fn checked_shapes_and_switch_thumb_positions_reach_the_software_renderer() {
    use halley_ui::{Color, TextSystem, software::PixelBuffer};
    let mut view = UiView::new("pixels").anchor(Anchor::TopLeft).content(
        Column::new("settings")
            .child(Checkbox::new("check", "Check").checked(true))
            .child(Switch::new("switch", "Switch"))
            .child(
                RadioGroup::new("group")
                    .selected("one")
                    .child(RadioOption::new("one", "One")),
            ),
    );
    let mut text = TextSystem::new();
    let theme = Theme::default();
    let bounds = Rect::new(0.0, 0.0, 250.0, 180.0);
    let first = view.prepare(bounds, &theme, &mut text).unwrap().clone();
    let thumb = |scene: &PreparedView| {
        scene
            .items
            .iter()
            .find_map(|p| match p {
                PaintItem::Card { key, rect, .. } if key == "pixels/switch/mark" => Some(*rect),
                _ => None,
            })
            .unwrap()
    };
    let mut pixels = PixelBuffer::new(250, 180).unwrap();
    pixels.surface().draw(&first, &mut text, 1.0);
    assert!(first.items.iter().any(|p| matches!(p, PaintItem::Text { key, text, .. } if key == "pixels/check/mark" && text == "✓")));
    assert!(
        first
            .items
            .iter()
            .any(|p| matches!(p, PaintItem::Card { key, .. } if key == "pixels/one/mark"))
    );
    let old = thumb(&first);
    let sample = |buffer: &PixelBuffer, point: Point| {
        let index = (point.y as usize * 250 + point.x as usize) * 4;
        buffer.pixels()[index..index + 4].to_vec()
    };
    let left_pixel = sample(&pixels, center(old));
    assert_eq!(left_pixel, vec![240, 245, 250, 255]);
    view.set_checked("switch", true).unwrap();
    let second = view.prepare(bounds, &theme, &mut text).unwrap().clone();
    pixels.surface().clear(Color::rgba(0.0, 0.0, 0.0, 0.0));
    pixels.surface().draw(&second, &mut text, 1.0);
    assert!(thumb(&second).origin.x > old.origin.x);
    assert_ne!(sample(&pixels, center(old)), left_pixel);
    assert_eq!(sample(&pixels, center(thumb(&second))), left_pixel);
}

#[test]
fn space_capture_cannot_act_as_pointer_capture_and_escape_cancels_a_pointer_press() {
    let mut view = view();
    let scene = prepare(&mut view);
    view.focus("notifications").unwrap();
    key(&mut view, Key::Space);
    assert!(
        view.handle_event(InputEvent::PointerUp {
            position: center(scene.rects["notifications"])
        })
        .is_empty()
    );
    assert!(
        view.handle_event(InputEvent::KeyUp { key: Key::Space })
            .is_empty()
    );
    assert_eq!(view.checked("notifications"), Some(true));
    let point = center(scene.rects["network"]);
    view.handle_event(InputEvent::PointerDown {
        position: point,
        shift: false,
    });
    key(&mut view, Key::Escape);
    assert!(
        view.handle_event(InputEvent::PointerUp { position: point })
            .is_empty()
    );
    assert_eq!(view.checked("network"), Some(false));
}
