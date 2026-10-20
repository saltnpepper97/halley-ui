use halley_ui::{InputEvent, Key, Point, ui::PaintItem};
use halley_ui_demo::{Appearance, Demo};
use std::time::Duration;

#[test]
fn motion_within_one_hover_target_does_not_request_more_frames() {
    let mut demo = Demo::new(Appearance::Light).unwrap();
    assert!(demo.needs_redraw());
    demo.render().unwrap();
    assert!(!demo.needs_redraw());
    let point = demo.control_point("balance").unwrap();
    demo.input(InputEvent::PointerMove { position: point })
        .unwrap();
    assert!(demo.needs_redraw());
    let before = demo.render().unwrap().pixels().to_vec();
    for i in 0..80 {
        demo.input(InputEvent::PointerMove {
            position: Point::new(point.x + (i % 10) as f32, point.y),
        })
        .unwrap();
        assert!(!demo.needs_redraw());
    }
    demo.advance(Duration::from_secs(1));
    assert!(!demo.needs_redraw());
    assert_eq!(demo.render().unwrap().pixels(), before);
    demo.stack.base_mut().invalidate();
    assert!(demo.needs_redraw());
    assert_eq!(demo.render().unwrap().pixels(), before);
}

#[test]
fn values_status_theme_and_resize_invalidate_cached_pixels() {
    let mut demo = Demo::new(Appearance::Light).unwrap();
    demo.render().unwrap();
    demo.focus("balance").unwrap();
    let initial = demo.render().unwrap().pixels().to_vec();
    demo.key(Key::Right).unwrap();
    assert!(demo.needs_redraw());
    let changed = demo.render().unwrap().pixels().to_vec();
    assert_ne!(changed, initial);
    assert!(!demo.needs_redraw());
    demo.set_status("Check complete");
    assert!(demo.needs_redraw());
    assert_ne!(demo.render().unwrap().pixels(), changed);
    demo.set_status("Check complete");
    assert!(!demo.needs_redraw());
    demo.set_appearance(Appearance::Dark);
    assert!(demo.needs_redraw());
    let pixel = &demo.render().unwrap().pixels()[..4];
    assert_eq!(pixel, &[22, 27, 35, 255]);
    demo.resize(560, 600, 1.0).unwrap();
    assert!(demo.needs_redraw());
    assert_eq!(demo.render().unwrap().pixels().len(), 560 * 600 * 4);
    demo.resize(560, 600, 1.0).unwrap();
    assert!(!demo.needs_redraw());
}

#[test]
fn motion_presents_its_final_frame_then_stops_requesting_work() {
    let mut demo = Demo::new(Appearance::Light).unwrap();
    demo.render().unwrap();
    demo.focus("autoplay").unwrap();
    demo.key(Key::Space).unwrap();
    demo.input(InputEvent::KeyUp { key: Key::Space }).unwrap();
    assert!(demo.stack.has_animations());
    demo.render().unwrap();
    demo.advance(Duration::from_millis(50));
    assert!(demo.needs_redraw());
    demo.render().unwrap();
    demo.advance(Duration::from_millis(110));
    assert!(!demo.stack.has_animations());
    assert!(demo.needs_redraw());
    demo.render().unwrap();
    assert!(!demo.needs_redraw());
    demo.advance(Duration::from_millis(50));
    assert!(!demo.needs_redraw());
    assert_eq!(demo.stack.base().checked("autoplay"), Some(false));
}

#[test]
fn focus_outlines_are_hidden_except_for_text_input_focus_borders() {
    for appearance in [Appearance::Light, Appearance::Dark] {
        let mut demo = Demo::new(appearance).unwrap();
        demo.render().unwrap();
        for key in [
            "theme-light",
            "components-tab",
            "volume",
            "balance",
            "autoplay",
            "normalize",
            "balanced",
        ] {
            demo.focus(key).unwrap();
            assert_eq!(demo.stack.base().focused(), Some(key));
            assert!(
                !demo.scene.items.iter().any(
                    |item| matches!(item, PaintItem::Card {key,..} if key.ends_with("/focus"))
                )
            );
        }
        demo.focus("name").unwrap();
        assert!(demo.scene.items.iter().any(|item| matches!(item, PaintItem::Card {key,style,..} if key.ends_with("/name") && style.border_width==2.0 && style.border==appearance.palette().accent)));
    }
}
