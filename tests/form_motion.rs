use halley_ui::{
    Anchor, Checkbox, Color, Dialog, InputEvent, Key, LayerEvent, LayerId, Modifiers, Rect, Size,
    Switch, Theme, UiEvent, UiStack, UiView,
    ui::{PaintItem, PreparedView},
};
use std::time::Duration;

fn prepare(view: &mut UiView) -> PreparedView {
    view.prepare_with_measure(
        Rect::new(0.0, 0.0, 300.0, 120.0),
        &Theme::default(),
        |_, _| Size::new(60.0, 16.0),
    )
    .unwrap()
    .clone()
}
fn thumb(scene: &PreparedView) -> Rect {
    scene
        .items
        .iter()
        .find_map(|item| match item {
            PaintItem::Card { key, rect, .. } if key.ends_with("/toggle/mark") => Some(*rect),
            _ => None,
        })
        .unwrap()
}
fn fill(scene: &PreparedView) -> Color {
    scene
        .items
        .iter()
        .find_map(|item| match item {
            PaintItem::Card { key, style, .. } if key.ends_with("/toggle/indicator") => {
                Some(style.fill)
            }
            _ => None,
        })
        .unwrap()
}
fn switch() -> UiView {
    UiView::new("motion")
        .anchor(Anchor::TopLeft)
        .content(Switch::new("toggle", "Switch"))
}

#[test]
fn checkbox_glyph_ink_is_centered_and_drawn_without_scaling_across_sizes_and_fonts() {
    for side in [12.0, 18.0, 22.0, 32.0, 64.0] {
        let mut view =
            UiView::new("check").content(Checkbox::new("check", "Checkbox").checked(true));
        let mut theme = Theme::default();
        theme.forms.indicator_size = side;
        theme.forms.switch_width = side.max(34.0);
        for family in ["sans-serif", "monospace"] {
            theme.font.family = family.into();
            let scene = view
                .prepare_with_measure(Rect::new(0.0, 0.0, 300.0, 150.0), &theme, |_, _| {
                    Size::new(80.0, 20.0)
                })
                .unwrap();
            let (rect, image) = scene
                .items
                .iter()
                .find_map(|item| match item {
                    PaintItem::Image {
                        key, rect, data, ..
                    } if key == "check/check/mark" => Some((rect, data)),
                    _ => None,
                })
                .unwrap();
            let mut left = image.width();
            let mut right = 0;
            let mut top = image.height();
            let mut bottom = 0;
            for (i, pixel) in image.pixels().chunks_exact(4).enumerate() {
                if pixel[3] > 0 {
                    let x = i as u32 % image.width();
                    let y = i as u32 / image.width();
                    left = left.min(x);
                    right = right.max(x);
                    top = top.min(y);
                    bottom = bottom.max(y);
                }
            }
            assert_eq!(
                (left, top, right + 1, bottom + 1),
                (0, 0, image.width(), image.height())
            );
            let indicator = scene
                .items
                .iter()
                .find_map(|item| match item {
                    PaintItem::Card { key, rect, .. } if key == "check/check/indicator" => {
                        Some(rect)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(
                (rect.origin.x + rect.size.width * 0.5 - indicator.origin.x - side * 0.5).abs()
                    <= 0.5
            );
            assert!(
                (rect.origin.y + rect.size.height * 0.5 - indicator.origin.y - side * 0.5).abs()
                    <= 0.5
            );
            assert_eq!(rect.size, image.size());
            assert_eq!(rect.origin.x, rect.origin.x.round());
            assert_eq!(rect.origin.y, rect.origin.y.round());
            let only_mark = PreparedView {
                items: scene
                    .items
                    .iter()
                    .filter(|item| {
                        matches!(item,
                PaintItem::Image { key, .. } if key == "check/check/mark")
                    })
                    .cloned()
                    .collect(),
                ..Default::default()
            };
            let mut pixels = halley_ui::software::PixelBuffer::new(300, 150).unwrap();
            pixels
                .surface()
                .draw(&only_mark, &mut halley_ui::TextSystem::new(), 1.0);
            for y in 0..image.height() as usize {
                for x in 0..image.width() as usize {
                    let source = (y * image.width() as usize + x) * 4;
                    let destination =
                        ((rect.origin.y as usize + y) * 300 + rect.origin.x as usize + x) * 4;
                    assert_eq!(
                        &pixels.pixels()[destination..destination + 4],
                        &image.pixels()[source..source + 4]
                    );
                }
            }
        }
    }
}

#[test]
fn switch_motion_advances_thumb_and_track_without_delaying_values_or_moving_hits() {
    let mut view = switch();
    view.set_switch_animation_duration(Duration::from_millis(160));
    let off = prepare(&mut view);
    assert!(!view.has_animations());
    view.set_checked("toggle", true).unwrap();
    let start = prepare(&mut view);
    assert_eq!(view.checked("toggle"), Some(true));
    assert_eq!(start.semantics[0].checked, Some(true));
    assert_eq!(thumb(&start), thumb(&off));
    assert_eq!(fill(&start), fill(&off));
    assert!(view.has_animations());
    assert!(!view.advance_animations(Duration::ZERO));
    assert!(view.advance_animations(Duration::from_millis(60)));
    let middle = prepare(&mut view);
    assert!(thumb(&middle).origin.x > thumb(&off).origin.x);
    assert_ne!(fill(&middle), fill(&off));
    assert_eq!(middle.rects, off.rects);
    assert_eq!(middle.controls[0].rect, off.controls[0].rect);
    assert!(view.advance_animations(Duration::from_secs(1)));
    let on = prepare(&mut view);
    assert!(thumb(&middle).origin.x < thumb(&on).origin.x);
    assert_eq!(fill(&on), Theme::default().button_pressed);
    assert!(!view.has_animations());
    assert!(!view.advance_animations(Duration::from_secs(1)));
    assert!(!view.needs_prepare());
}

#[test]
fn rapid_reversal_and_timing_changes_continue_from_the_presented_position_and_color() {
    let mut view = switch();
    view.set_switch_animation_duration(Duration::from_millis(160));
    let off = prepare(&mut view);
    view.set_checked("toggle", true).unwrap();
    prepare(&mut view);
    view.advance_animations(Duration::from_millis(60));
    let forward = prepare(&mut view);
    view.set_checked("toggle", false).unwrap();
    let reversed = prepare(&mut view);
    assert_eq!(thumb(&forward), thumb(&reversed));
    assert_eq!(fill(&forward), fill(&reversed));
    view.advance_animations(Duration::from_millis(20));
    let backward = prepare(&mut view);
    assert!(thumb(&backward).origin.x < thumb(&forward).origin.x);
    assert!(thumb(&backward).origin.x > thumb(&off).origin.x);
    view.set_switch_animation_duration(Duration::from_millis(400));
    let retimed = prepare(&mut view);
    assert_eq!(thumb(&retimed), thumb(&backward));
    view.advance_animations(Duration::from_millis(20));
    let slower = prepare(&mut view);
    assert!(thumb(&slower).origin.x <= thumb(&backward).origin.x);
    view.set_switch_animation_duration(Duration::ZERO);
    let snapped = prepare(&mut view);
    assert_eq!(thumb(&snapped), thumb(&off));
    assert!(!view.has_animations());
    assert_eq!(view.checked("toggle"), Some(false));
}

#[test]
fn initial_values_do_not_animate_and_removed_switches_cannot_keep_a_frame_loop_alive() {
    let mut view = UiView::new("initial").content(Switch::new("toggle", "On").checked(true));
    view.set_switch_animation_duration(Duration::from_millis(160));
    prepare(&mut view);
    assert!(!view.has_animations());
    view.set_checked("toggle", false).unwrap();
    assert!(view.has_animations());
    view.set_content(Switch::new("toggle", "New label").checked(true));
    prepare(&mut view);
    assert!(view.has_animations());
    assert_eq!(view.checked("toggle"), Some(false));
    view.set_content(Checkbox::new("check", "No switch"));
    assert!(!view.advance_animations(Duration::from_millis(10)));
    assert!(!view.has_animations());
}

#[test]
fn stack_layers_inherit_motion_and_emit_one_logical_change_while_frames_advance() {
    let mut stack = UiStack::new(UiView::new("base"));
    stack.set_switch_animation_duration(Duration::from_millis(160));
    let render = |stack: &mut UiStack| {
        stack
            .prepare_with_measure(
                Rect::new(0.0, 0.0, 400.0, 200.0),
                &Theme::default(),
                |_, _| Size::new(60.0, 16.0),
            )
            .unwrap()
            .clone()
    };
    render(&mut stack);
    stack
        .open(Dialog::new("settings").child(Switch::new("toggle", "Toggle")))
        .unwrap();
    let off = render(&mut stack);
    stack.handle_event(InputEvent::KeyDown {
        key: Key::Space,
        modifiers: Modifiers::default(),
    });
    assert_eq!(
        stack.handle_event(InputEvent::KeyUp { key: Key::Space }),
        vec![LayerEvent::View {
            layer: LayerId::overlay("settings"),
            event: UiEvent::CheckedChanged {
                key: "toggle".into(),
                checked: true
            },
        }]
    );
    assert!(stack.has_animations());
    stack.advance_animations(Duration::from_millis(60));
    let moving = render(&mut stack);
    assert!(thumb(&moving).origin.x > thumb(&off).origin.x);
    assert!(stack.take_events().iter().all(|event| !matches!(
        event,
        LayerEvent::View {
            event: UiEvent::CheckedChanged { .. },
            ..
        }
    )));
    stack.close_top();
    assert!(!stack.has_animations());
}
