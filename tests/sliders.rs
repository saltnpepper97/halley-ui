use halley_ui::{
    Anchor, Button, Column, Dialog, InputEvent, InteractionError, Key, Label, LayerEvent, LayerId,
    Modifiers, Point, Rect, Scroll, Size, Slider, Style, StyleSheet, Theme, UiEvent, UiStack,
    UiView,
    ui::{ControlKind, LayoutError, PaintItem, PreparedView},
};

fn slider() -> Slider {
    Slider::new("level", 0.0, 100.0)
        .label("Volume")
        .value(50.0)
        .step(5.0)
}
fn view() -> UiView {
    UiView::new("settings")
        .anchor(Anchor::TopLeft)
        .content(slider())
}
fn prepare_with(view: &mut UiView, theme: &Theme) -> PreparedView {
    view.prepare_with_measure(Rect::new(20.0, 30.0, 480.0, 320.0), theme, |_, text| {
        Size::new(text.len() as f32 * 8.0, 16.0)
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
fn changed(value: f64) -> UiEvent {
    UiEvent::SliderChanged {
        key: "level".into(),
        value,
    }
}
fn band(scene: &PreparedView) -> Rect {
    scene
        .controls
        .iter()
        .find(|c| matches!(c.kind, ControlKind::Slider { .. }))
        .unwrap()
        .content
}
fn point(scene: &PreparedView, fraction: f32) -> Point {
    let control = scene
        .controls
        .iter()
        .find(|c| matches!(c.kind, ControlKind::Slider { .. }))
        .unwrap();
    let ControlKind::Slider { thumb_size, .. } = control.kind else {
        unreachable!()
    };
    Point::new(
        control.content.origin.x
            + thumb_size * 0.5
            + (control.content.size.width - thumb_size) * fraction,
        control.content.origin.y + control.content.size.height * 0.5,
    )
}
fn down(view: &mut UiView, point: Point) -> Vec<UiEvent> {
    view.handle_event(InputEvent::PointerDown {
        position: point,
        shift: false,
    })
}
fn moving(view: &mut UiView, point: Point) -> Vec<UiEvent> {
    view.handle_event(InputEvent::PointerMove { position: point })
}
fn up(view: &mut UiView, point: Point) -> Vec<UiEvent> {
    view.handle_event(InputEvent::PointerUp { position: point })
}

#[test]
fn track_press_drag_and_release_emit_only_actual_changes_and_clamp_outside() {
    let mut view = view();
    let scene = prepare(&mut view);
    assert!(down(&mut view, point(&scene, 0.75)).contains(&changed(75.0)));
    assert_eq!(view.focused(), Some("level"));
    assert!(moving(&mut view, point(&scene, 0.751)).is_empty());
    assert_eq!(
        moving(&mut view, Point::new(9999.0, -50.0)),
        vec![changed(100.0)]
    );
    assert_eq!(up(&mut view, Point::new(-200.0, 900.0)), vec![changed(0.0)]);
    assert!(moving(&mut view, point(&scene, 0.5)).is_empty());
    assert_eq!(view.slider_value("level"), Some(0.0));
    assert!(view.needs_prepare());
}

#[test]
fn thumb_grab_keeps_its_offset_and_label_press_only_focuses() {
    let mut view = view();
    let scene = prepare(&mut view);
    let mut grab = point(&scene, 0.5);
    grab.x += 6.0;
    assert!(
        !down(&mut view, grab)
            .iter()
            .any(|e| matches!(e, UiEvent::SliderChanged { .. }))
    );
    assert!(moving(&mut view, grab).is_empty());
    grab.x += (band(&scene).size.width - 18.0) * 0.25;
    assert_eq!(moving(&mut view, grab), vec![changed(75.0)]);
    up(&mut view, grab);
    let label = Point::new(
        band(&scene).origin.x + 5.0,
        scene.rects["level"].origin.y + 8.0,
    );
    assert!(down(&mut view, label).is_empty());
    assert!(moving(&mut view, point(&scene, 0.0)).is_empty());
    assert!(up(&mut view, point(&scene, 0.0)).is_empty());
    assert_eq!(view.slider_value("level"), Some(75.0));
}

#[test]
fn grabbing_a_continuous_thumb_does_not_roundtrip_its_value_through_pixel_precision() {
    let value = 0.123456789123;
    let mut view = UiView::new("precision").content(Slider::new("level", 0.0, 1.0).value(value));
    let scene = prepare(&mut view);
    let grab = point(&scene, value as f32);
    assert!(
        !down(&mut view, grab)
            .iter()
            .any(|e| matches!(e, UiEvent::SliderChanged { .. }))
    );
    assert!(moving(&mut view, grab).is_empty());
    assert!(up(&mut view, grab).is_empty());
    assert_eq!(view.slider_value("level"), Some(value));
}

#[test]
fn keyboard_arrows_pages_and_endpoints_respect_step_and_modifiers() {
    let mut view = view();
    prepare(&mut view);
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("level"));
    for (command, expected) in [
        (Key::Right, 55.0),
        (Key::Up, 60.0),
        (Key::Left, 55.0),
        (Key::Down, 50.0),
        (Key::PageUp, 60.0),
        (Key::PageDown, 50.0),
        (Key::Home, 0.0),
        (Key::End, 100.0),
    ] {
        assert_eq!(key(&mut view, command), vec![changed(expected)]);
    }
    assert!(key(&mut view, Key::Right).is_empty());
    for command in [Key::Space, Key::Enter] {
        assert!(key(&mut view, command).is_empty());
    }
    for modifiers in [
        Modifiers {
            control: true,
            ..Default::default()
        },
        Modifiers {
            alt: true,
            ..Default::default()
        },
    ] {
        assert!(
            view.handle_event(InputEvent::KeyDown {
                key: Key::Home,
                modifiers
            })
            .is_empty()
        );
    }
    assert_eq!(view.slider_value("level"), Some(100.0));
}

#[test]
fn fractional_nondividing_and_oversized_steps_have_reachable_endpoints() {
    let mut view =
        UiView::new("steps").content(Slider::new("level", -1.0, 1.0).step(0.3).value(-0.4));
    prepare(&mut view);
    view.focus("level").unwrap();
    key(&mut view, Key::Right);
    assert!((view.slider_value("level").unwrap() + 0.1).abs() < 1e-12);
    key(&mut view, Key::End);
    key(&mut view, Key::Left);
    assert!((view.slider_value("level").unwrap() - 0.8).abs() < 1e-12);
    key(&mut view, Key::Right);
    assert_eq!(view.slider_value("level"), Some(1.0));
    view.set_content(Slider::new("level", 0.0, 10.0).step(30.0));
    prepare(&mut view);
    key(&mut view, Key::Home);
    key(&mut view, Key::Right);
    assert_eq!(view.slider_value("level"), Some(10.0));
    key(&mut view, Key::Left);
    assert_eq!(view.slider_value("level"), Some(0.0));
}

#[test]
fn decimal_endpoints_always_step_back_and_subnormal_ranges_remain_usable() {
    for step in [0.01, 0.03, 0.05, 0.1, 0.2, 0.3] {
        for ticks in 1..20 {
            let max = step * ticks as f64;
            let mut view = UiView::new("decimals")
                .content(Slider::new("level", 0.0, max).step(step).value(max));
            prepare(&mut view);
            view.focus("level").unwrap();
            key(&mut view, Key::Left);
            let value = view.slider_value("level").unwrap();
            assert!(value < max, "step {step}, max {max}");
            assert!((value - step * (ticks - 1) as f64).abs() < 1e-12);
        }
    }
    let max = f64::from_bits(1);
    let mut view = UiView::new("tiny-range").content(Slider::new("level", 0.0, max));
    prepare(&mut view);
    view.focus("level").unwrap();
    assert_eq!(key(&mut view, Key::Right), vec![changed(max)]);
}

#[test]
fn software_pixels_follow_the_value_and_theme_without_resizing_the_control() {
    use halley_ui::{Color, TextSystem, software::PixelBuffer};
    let mut theme = Theme::default();
    theme.styles.slider.normal = Style {
        fill: Some(Color::rgb8(30, 40, 50)),
        text: Some(Color::rgb8(230, 240, 250)),
        selection: Some(Color::rgb8(20, 150, 80)),
        ..Default::default()
    };
    let mut view = UiView::new("pixels")
        .anchor(Anchor::TopLeft)
        .content(Slider::new("level", 0.0, 100.0).value(25.0));
    let first = prepare_with(&mut view, &theme);
    let paint = |scene: &PreparedView| {
        let mut pixels = PixelBuffer::new(520, 400).unwrap();
        pixels.surface().draw(scene, &mut TextSystem::new(), 1.0);
        pixels
    };
    let pixel = |pixels: &PixelBuffer, p: Point| {
        let index = (p.y.floor() as usize * 520 + p.x.floor() as usize) * 4;
        pixels.pixels()[index..index + 4].to_vec()
    };
    let before = paint(&first);
    assert_eq!(pixel(&before, point(&first, 0.1)), vec![20, 150, 80, 255]);
    assert_eq!(pixel(&before, point(&first, 0.5)), vec![30, 40, 50, 255]);
    view.set_slider_value("level", 75.0).unwrap();
    let second = prepare_with(&mut view, &theme);
    assert_eq!(first.rects["level"], second.rects["level"]);
    let after = paint(&second);
    assert_eq!(pixel(&after, point(&second, 0.5)), vec![20, 150, 80, 255]);
    assert_eq!(
        pixel(&after, point(&second, 0.75)),
        vec![230, 240, 250, 255]
    );
    assert_eq!(pixel(&after, point(&second, 0.9)), vec![30, 40, 50, 255]);
}

#[test]
fn values_survive_rebuilds_resnap_for_new_ranges_and_reset_after_removal() {
    let mut view = view();
    prepare(&mut view);
    assert!(view.set_slider_value("level", 73.0).unwrap());
    assert_eq!(view.slider_value("level"), Some(75.0));
    view.set_content(slider().value(10.0).label("New label"));
    prepare(&mut view);
    assert_eq!(view.slider_value("level"), Some(75.0));
    view.set_content(Slider::new("level", 0.0, 30.0).step(4.0));
    prepare(&mut view);
    assert_eq!(view.slider_value("level"), Some(30.0));
    view.set_content(Label::new("empty", "Empty"));
    prepare(&mut view);
    assert_eq!(view.slider_value("level"), None);
    view.set_content(slider());
    prepare(&mut view);
    assert_eq!(view.slider_value("level"), Some(50.0));
}

#[test]
fn disabled_sliders_skip_focus_and_input_but_accept_silent_updates() {
    let mut view = UiView::new("disabled").content(
        Column::new("form")
            .child(slider().disabled(true))
            .child(Button::new("done", "Done")),
    );
    let scene = prepare(&mut view);
    assert!(down(&mut view, point(&scene, 0.8)).is_empty());
    assert!(up(&mut view, point(&scene, 0.8)).is_empty());
    key(&mut view, Key::Tab);
    assert_eq!(view.focused(), Some("done"));
    assert!(matches!(
        view.focus("level"),
        Err(InteractionError::Disabled(_))
    ));
    assert!(view.set_slider_value("level", 999.0).unwrap());
    assert_eq!(view.slider_value("level"), Some(100.0));
    assert!(!view.set_slider_value("level", 100.0).unwrap());
    assert!(matches!(
        view.set_slider_value("missing", 1.0),
        Err(InteractionError::NotSlider(_))
    ));
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            view.set_slider_value("level", value),
            Err(InteractionError::InvalidNumericValue)
        ));
    }
    assert_eq!(view.slider_value("level"), Some(100.0));
}

#[test]
fn captures_cancel_on_escape_blur_pointer_cancel_focus_and_content_changes() {
    for cancel in [
        InputEvent::PointerCancel,
        InputEvent::WindowFocus(false),
        InputEvent::KeyDown {
            key: Key::Escape,
            modifiers: Modifiers::default(),
        },
        InputEvent::KeyDown {
            key: Key::Tab,
            modifiers: Modifiers::default(),
        },
    ] {
        let mut view = view();
        view.set_content(
            Column::new("form")
                .child(slider())
                .child(Button::new("done", "Done")),
        );
        let scene = prepare(&mut view);
        down(&mut view, point(&scene, 0.5));
        view.handle_event(cancel);
        view.handle_event(InputEvent::WindowFocus(true));
        moving(&mut view, point(&scene, 0.0));
        up(&mut view, point(&scene, 0.0));
        assert_eq!(view.slider_value("level"), Some(50.0));
    }
    let mut view = view();
    let scene = prepare(&mut view);
    down(&mut view, point(&scene, 0.5));
    view.set_slider_value("level", 80.0).unwrap();
    moving(&mut view, point(&scene, 0.0));
    up(&mut view, point(&scene, 0.0));
    assert_eq!(view.slider_value("level"), Some(80.0));
    down(&mut view, point(&scene, 0.5));
    view.set_content(slider().disabled(true));
    let disabled = prepare(&mut view);
    moving(&mut view, point(&disabled, 0.0));
    up(&mut view, point(&disabled, 0.0));
    assert_eq!(view.slider_value("level"), Some(50.0));
}

#[test]
fn translated_input_and_clipped_hits_agree_with_paint() {
    let mut view = view();
    let original = prepare(&mut view);
    let scene = original.translated(Point::new(100.0, 70.0));
    assert!(
        view.handle_event_in(
            &scene,
            InputEvent::PointerDown {
                position: point(&scene, 0.8),
                shift: false
            }
        )
        .contains(&changed(80.0))
    );
    view.handle_event_in(
        &scene,
        InputEvent::PointerUp {
            position: point(&scene, 0.8),
        },
    );
    let mut clipped = scene.clone();
    let point = point(&clipped, 0.0);
    clipped
        .controls
        .iter_mut()
        .for_each(|c| c.clip = Rect::new(0.0, 0.0, 1.0, 1.0));
    assert!(
        !view
            .handle_event_in(
                &clipped,
                InputEvent::PointerDown {
                    position: point,
                    shift: false
                }
            )
            .iter()
            .any(|e| matches!(e, UiEvent::SliderChanged { .. }))
    );
    assert_eq!(view.slider_value("level"), Some(80.0));
    view.set_content(slider().label("Changed"));
    prepare(&mut view);
    assert!(
        view.handle_event_in(
            &original,
            InputEvent::PointerDown {
                position: point,
                shift: false
            }
        )
        .is_empty()
    );
}

#[test]
fn invalid_specs_and_theme_metrics_fail_preparation_without_panics() {
    for slider in [
        Slider::new("bad", 1.0, 1.0),
        Slider::new("bad", 2.0, 1.0),
        Slider::new("bad", f64::NEG_INFINITY, 1.0),
        Slider::new("bad", -f64::MAX, f64::MAX),
        Slider::new("bad", 0.0, 1.0).step(0.0),
        Slider::new("bad", 0.0, 1.0).step(-1.0),
        Slider::new("bad", 0.0, 1.0).step(f64::NAN),
        Slider::new("bad", 0.0, 1.0).value(f64::NAN),
    ] {
        assert!(matches!(
            UiView::new("invalid").content(slider).prepare_with_measure(
                Rect::new(0.0, 0.0, 400.0, 200.0),
                &Theme::default(),
                |_, _| Size::default()
            ),
            Err(LayoutError::InvalidSlider(_))
        ));
    }
    let mut view = view();
    let mut theme = Theme::default();
    theme.forms.slider_thumb_size = 0.0;
    assert!(
        view.prepare_with_measure(Rect::new(0.0, 0.0, 400.0, 200.0), &theme, |_, _| {
            Size::default()
        })
        .is_err()
    );
}

#[test]
fn continuous_ranges_theme_changes_and_tiny_layouts_produce_finite_geometry() {
    let mut view =
        UiView::new("continuous").content(Slider::new("level", -10.0, 10.0).value(0.125));
    let scene = prepare(&mut view);
    view.focus("level").unwrap();
    key(&mut view, Key::Right);
    assert!((view.slider_value("level").unwrap() - 0.325).abs() < 1e-12);
    let mut theme = Theme::default();
    theme.forms.slider_width = 240.0;
    theme.forms.slider_thumb_size = 24.0;
    theme.styles.slider = StyleSheet {
        normal: Style {
            selection: Some(halley_ui::Color::rgb8(20, 150, 80)),
            ..Default::default()
        },
        ..Default::default()
    };
    let styled = prepare_with(&mut view, &theme);
    assert!(styled.rects["level"].size.width > scene.rects["level"].size.width);
    assert_eq!(view.slider_value("level"), Some(0.325));
    view.set_content(Slider::new("level", 0.0, 100.0).width(6.0).height(2.0));
    let tiny = prepare(&mut view);
    for item in tiny.items {
        if let PaintItem::Card { rect, .. } = item {
            assert!(rect.size.width.is_finite() && rect.size.width >= 0.0);
            assert!(rect.size.height.is_finite() && rect.size.height >= 0.0);
        }
    }
}

#[test]
fn scroll_reveals_focused_slider_and_wheel_does_not_change_its_value() {
    let mut view = UiView::new("scroll").anchor(Anchor::TopLeft).content(
        Scroll::new("scroll").height(60.0).width(240.0).child(
            Column::new("content")
                .child(Label::new("spacer", "Spacer").height(160.0))
                .child(slider()),
        ),
    );
    prepare(&mut view);
    view.focus("level").unwrap();
    let scene = prepare(&mut view);
    assert!(scene.scrolls[0].offset.y > 0.0);
    view.handle_event(InputEvent::Wheel {
        position: point(&scene, 0.5),
        delta: Point::new(0.0, -20.0),
    });
    assert_eq!(view.slider_value("level"), Some(50.0));
}

fn prepare_stack(stack: &mut UiStack) -> PreparedView {
    stack
        .prepare_with_measure(
            Rect::new(0.0, 0.0, 500.0, 400.0),
            &Theme::default(),
            |_, text| Size::new(text.len() as f32 * 8.0, 16.0),
        )
        .unwrap()
        .clone()
}
#[test]
fn overlays_scope_numeric_events_and_cancel_underlying_drags() {
    let mut stack = UiStack::new(view());
    let base = prepare_stack(&mut stack);
    stack.handle_event(InputEvent::PointerDown {
        position: point(&base, 0.5),
        shift: false,
    });
    stack
        .open(Dialog::new("dialog").child(slider().value(25.0)))
        .unwrap();
    prepare_stack(&mut stack);
    assert_eq!(
        stack.handle_event(InputEvent::KeyDown {
            key: Key::Right,
            modifiers: Modifiers::default()
        }),
        vec![LayerEvent::View {
            layer: LayerId::overlay("dialog"),
            event: changed(30.0)
        }]
    );
    assert!(
        stack
            .set_slider_value(&LayerId::Base, "level", 80.0)
            .unwrap()
    );
    assert_eq!(stack.base().slider_value("level"), Some(80.0));
    stack.close_top();
    prepare_stack(&mut stack);
    stack.handle_event(InputEvent::PointerMove {
        position: point(&base, 0.0),
    });
    stack.handle_event(InputEvent::PointerUp {
        position: point(&base, 0.0),
    });
    assert_eq!(stack.base().slider_value("level"), Some(80.0));
}

#[cfg(feature = "accessibility")]
#[test]
fn numeric_accessibility_exposes_range_and_routes_value_and_step_actions() {
    use halley_ui::accessibility::{
        AccessibilityTree, AccessibleAction,
        accesskit::{Action, ActionData, ActionRequest, Role, TreeId},
    };
    let mut view = view();
    let scene = prepare(&mut view);
    let mut tree = AccessibilityTree::default();
    let update = tree.update(&scene, "Settings");
    let consumer = accesskit_consumer::Tree::new(update.clone(), true);
    assert!(consumer.state().root().children().count() > 0);
    let (id, node) = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Slider)
        .unwrap();
    assert_eq!(node.label(), Some("Volume"));
    assert_eq!(node.numeric_value(), Some(50.0));
    assert_eq!(node.min_numeric_value(), Some(0.0));
    assert_eq!(node.max_numeric_value(), Some(100.0));
    assert_eq!(node.numeric_value_step(), Some(5.0));
    for (action, data, expected) in [
        (Action::SetValue, Some(ActionData::NumericValue(73.0)), 75.0),
        (Action::Increment, None, 80.0),
        (Action::Decrement, None, 75.0),
    ] {
        assert!(node.supports_action(action));
        let action = tree
            .action(ActionRequest {
                action,
                target_tree: TreeId::ROOT,
                target_node: *id,
                data,
            })
            .unwrap();
        assert_eq!(view.handle_accessibility(action), vec![changed(expected)]);
    }
    assert!(
        view.handle_accessibility(AccessibleAction::SetNumericValue {
            key: "level".into(),
            value: f64::NAN
        })
        .is_empty()
    );
    view.set_content(slider().disabled(true));
    let disabled = prepare(&mut view);
    let update = tree.update(&disabled, "Settings");
    let (_, node) = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::Slider)
        .unwrap();
    assert!(node.is_disabled());
    assert_eq!(node.numeric_value(), Some(75.0));
    assert!(!node.supports_action(Action::SetValue));
    assert!(
        view.handle_accessibility(AccessibleAction::AdjustNumericValue {
            key: "level".into(),
            increase: true
        })
        .is_empty()
    );
    let mut stack = UiStack::new(UiView::new("base"));
    prepare_stack(&mut stack);
    stack.open(Dialog::new("dialog").child(slider())).unwrap();
    let scene = prepare_stack(&mut stack);
    let key = scene
        .semantics
        .iter()
        .find(|s| s.numeric_value.is_some())
        .unwrap()
        .key
        .clone();
    assert_eq!(
        stack.handle_accessibility(AccessibleAction::SetNumericValue { key, value: 90.0 }),
        vec![LayerEvent::View {
            layer: LayerId::overlay("dialog"),
            event: changed(90.0)
        }]
    );
}
