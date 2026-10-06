use halley_ui::assets::ImageData;
use halley_ui::software::{PixelBuffer, PixelFormat, Surface};
use halley_ui::ui::{CardStyle, LayoutError, PaintItem};
use halley_ui::{
    ActionId, Anchor, Button, Card, Color, Column, Font, Image, Label, Point, Rect, Row, Size,
    TextOverflow, TextSystem, Theme, UiView,
};
use std::{cell::Cell, sync::Arc};

fn metrics(_: &Font, text: &str) -> Size {
    Size::new(text.chars().count() as f32 * 10.0, 12.0)
}

#[test]
fn anchors_padding_and_output_origins_share_one_layout() {
    let bounds = Rect::new(100.0, -50.0, 500.0, 300.0);
    for (anchor, expected) in [
        (Anchor::TopLeft, Point::new(124.0, -26.0)),
        (Anchor::TopCenter, Point::new(309.0, -26.0)),
        (Anchor::TopRight, Point::new(494.0, -26.0)),
        (Anchor::BottomLeft, Point::new(124.0, 198.0)),
        (Anchor::BottomCenter, Point::new(309.0, 198.0)),
        (Anchor::BottomRight, Point::new(494.0, 198.0)),
    ] {
        let mut view = UiView::new("notice").anchor(anchor).margin(24.0).content(
            Card::new("card")
                .padding_xy(16.0, 8.0)
                .child(Label::new("label", "Hello")),
        );
        let scene = view
            .prepare_with_measure(bounds, &Theme::default(), metrics)
            .unwrap();
        assert_eq!(scene.bounds.origin, expected);
        assert_eq!(scene.bounds.size, Size::new(82.0, 28.0));
        assert_eq!(
            scene.rects["label"].origin,
            Point::new(expected.x + 16.0, expected.y + 8.0)
        );
    }
}

#[test]
fn nested_layout_gaps_actions_and_translated_hits_agree() {
    let mut view = UiView::new("dialog").anchor(Anchor::TopLeft).content(
        Card::new("card").padding(16.0).child(
            Column::new("content")
                .gap(12.0)
                .child(Label::new("title", "Saved"))
                .child(
                    Row::new("actions")
                        .gap(8.0)
                        .child(
                            Button::new("copy", "Copy")
                                .width(100.0)
                                .height(30.0)
                                .action(ActionId::new("copy-action")),
                        )
                        .child(Button::new("open", "Open").width(100.0).height(30.0)),
                ),
        ),
    );
    let scene = view
        .prepare_with_measure(
            Rect::new(0.0, 0.0, 600.0, 400.0),
            &Theme::default(),
            metrics,
        )
        .unwrap();
    assert_eq!(
        scene.rects["open"].origin.x - scene.rects["copy"].origin.x,
        108.0
    );
    let copy = scene.rects["copy"];
    assert_eq!(
        scene
            .hit(Point::new(copy.origin.x + 5.0, copy.origin.y + 5.0))
            .unwrap()
            .action
            .0,
        "copy-action"
    );
    let moved = scene.translated(Point::new(33.0, -7.0));
    let point = Point::new(copy.origin.x + 38.0, copy.origin.y - 2.0);
    assert_eq!(moved.hit(point).unwrap().action.0, "copy-action");
    assert!(!moved.hit(Point::new(-100.0, -100.0)).is_some());
}

#[test]
fn layout_is_reused_and_font_or_content_changes_invalidate_it() {
    let calls = Cell::new(0);
    let mut measure = |font: &Font, text: &str| {
        calls.set(calls.get() + 1);
        metrics(font, text)
    };
    let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
    let mut theme = Theme::default();
    let mut view = UiView::new("notice").content(Label::new("label", "Hello"));
    view.prepare_with_measure(bounds, &theme, &mut measure)
        .unwrap();
    let first = calls.get();
    view.prepare_with_measure(bounds, &theme, &mut measure)
        .unwrap();
    assert_eq!(calls.get(), first);
    theme.font.size += 1;
    view.prepare_with_measure(bounds, &theme, &mut measure)
        .unwrap();
    assert!(calls.get() > first);
    view.set_content(Label::new("label", "A longer message"));
    let size = view
        .prepare_with_measure(bounds, &theme, &mut measure)
        .unwrap()
        .bounds
        .size;
    assert_eq!(size.width, 160.0);
}

#[test]
fn duplicate_keys_fail_and_disabled_buttons_do_not_activate() {
    let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
    let mut view = UiView::new("duplicate").content(
        Row::new("row")
            .child(Label::new("same", "a"))
            .child(Label::new("same", "b")),
    );
    assert!(matches!(
        view.prepare_with_measure(bounds, &Theme::default(), metrics),
        Err(LayoutError::DuplicateKey(_))
    ));
    let mut view = UiView::new("disabled")
        .anchor(Anchor::TopLeft)
        .content(Button::new("button", "No").disabled(true));
    let scene = view
        .prepare_with_measure(bounds, &Theme::default(), metrics)
        .unwrap();
    assert!(scene.hit(Point::new(10.0, 10.0)).is_none());
}

#[test]
fn constrained_text_and_images_fit_without_changing_aspect() {
    let image = Arc::new(ImageData::from_rgba(2, 1, vec![255; 8].into()).unwrap());
    let mut view = UiView::new("small").anchor(Anchor::TopLeft).content(
        Column::new("column")
            .max_width(80.0)
            .child(
                Label::new("title", "abcdefghijklmnopqrstuvwxyz")
                    .overflow(TextOverflow::EllipsisMiddle),
            )
            .child(Image::new("image", image).width(80.0).height(80.0)),
    );
    let scene = view
        .prepare_with_measure(
            Rect::new(0.0, 0.0, 300.0, 300.0),
            &Theme::default(),
            metrics,
        )
        .unwrap();
    assert!(scene.bounds.size.width <= 80.0);
    assert!(scene.items.iter().any(|item| matches!(item, PaintItem::Text { text, rect, .. } if text.contains('…') && rect.size.width <= 80.0)), "{scene:?}");
    assert!(scene.items.iter().any(
        |item| matches!(item, PaintItem::Image { rect, .. } if rect.size == Size::new(80.0, 40.0))
    ));
}

#[test]
fn invalid_buffer_geometry_is_rejected_and_bgra_stride_is_preserved() {
    let mut pixels = vec![0xa5; 32];
    assert!(Surface::new(&mut pixels, 3, 2, 8, PixelFormat::Bgra).is_err());
    assert!(Surface::new(&mut pixels[..8], 2, 2, 8, PixelFormat::Bgra).is_err());
    let mut surface = Surface::new(&mut pixels, 2, 2, 16, PixelFormat::Bgra).unwrap();
    surface.clear(Color::rgba(1.0, 0.0, 0.0, 0.5));
    assert_eq!(&pixels[..4], &[0, 0, 128, 128]);
    assert_eq!(&pixels[8..16], &[0xa5; 8]);
}

#[test]
fn software_draws_cards_text_and_svg_icons_as_premultiplied_pixels() {
    let icon = Arc::new(ImageData::from_svg(br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#00ff00"/></svg>"##, 16).unwrap());
    let mut text = TextSystem::new();
    let mut view = UiView::new("notice").anchor(Anchor::TopLeft).content(
        Card::new("card")
            .padding(10.0)
            .style(CardStyle {
                fill: Color::rgba(0.3, 0.2, 0.1, 0.5),
                ..Theme::default().card
            })
            .child(
                Row::new("row")
                    .gap(8.0)
                    .child(Image::new("icon", icon).width(16.0).height(16.0))
                    .child(Label::new("message", "Halley")),
            ),
    );
    let scene = view
        .prepare(
            Rect::new(0.0, 0.0, 240.0, 80.0),
            &Theme::default(),
            &mut text,
        )
        .unwrap();
    let mut buffer = PixelBuffer::new(240, 80).unwrap();
    buffer.surface().draw(scene, &mut text, 0.5);
    assert!(
        buffer
            .pixels()
            .chunks_exact(4)
            .any(|p| p[1] > p[0] && p[3] > 0)
    );
    assert!(
        buffer
            .pixels()
            .chunks_exact(4)
            .all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3])
    );
}

#[test]
fn text_cache_and_trailing_glyph_padding_survive_extraction() {
    let mut text = TextSystem::new();
    let font = Font::default();
    let first = text
        .raster(&font, "Build a cluster", [80, 160, 240])
        .unwrap();
    let second = text
        .raster(&font, "Build a cluster", [80, 160, 240])
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(first.pixels.chunks_exact(4).any(|p| p[3] > 0));
    for y in 0..first.height {
        assert_eq!(
            first.pixels[((y * first.width + first.width - 1) * 4 + 3) as usize],
            0
        );
    }
    let (fitted, size) = text.fit(
        &font,
        "A very long path with a combining character e\u{301}",
        90.0,
        TextOverflow::EllipsisMiddle,
    );
    assert!(size.width <= 90.0);
    assert!(fitted.contains('…'));
}

#[test]
fn repeated_button_updates_restore_normal_fill_and_do_not_accumulate_alpha() {
    let theme = Theme::default();
    let mut view = UiView::new("buttons").content(
        Row::new("row")
            .child(Button::new("enabled", "Yes"))
            .child(Button::new("disabled", "No").disabled(true)),
    );
    let mut scene = view
        .prepare_with_measure(Rect::new(0.0, 0.0, 200.0, 80.0), &theme, metrics)
        .unwrap()
        .clone();
    scene.button_visuals(&theme, Some("buttons/enabled"), None);
    scene.button_visuals(&theme, None, None);
    scene.button_visuals(&theme, None, None);
    for item in &scene.items {
        if let PaintItem::Card { key, style, .. } = item {
            if key == "buttons/enabled" {
                assert_eq!(style.fill, theme.button.fill);
            }
            if key == "buttons/disabled" {
                assert_eq!(style.fill.a, theme.button.fill.a * 0.5);
            }
        }
    }
}

#[test]
fn container_clip_limits_both_paint_and_translated_input() {
    let mut view = UiView::new("clipped").anchor(Anchor::TopLeft).content(
        Card::new("card")
            .width(80.0)
            .height(30.0)
            .padding(10.0)
            .child(Button::new("button", "Overflow").height(100.0)),
    );
    let scene = view
        .prepare_with_measure(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            &Theme::default(),
            metrics,
        )
        .unwrap();
    let clip = Rect::new(10.0, 10.0, 60.0, 10.0);
    assert_eq!(scene.hits[0].clip, scene.hits[0].rect.intersection(clip));
    assert!(scene.hit(Point::new(15.0, 25.0)).is_none());
    let moved = scene.translated(Point::new(20.0, 30.0));
    assert!(moved.hit(Point::new(35.0, 55.0)).is_none());
    for item in &scene.items {
        match item {
            PaintItem::Card {
                key, clip: actual, ..
            } if key == "clipped/button" => {
                assert_eq!(*actual, clip);
            }
            PaintItem::Text { clip: actual, .. } => {
                assert_eq!(*actual, actual.intersection(clip));
            }
            _ => {}
        }
    }
    let mut pixels = PixelBuffer::new(100, 100).unwrap();
    pixels.surface().draw(scene, &mut TextSystem::new(), 1.0);
    assert!(
        pixels
            .pixels()
            .chunks_exact(400)
            .skip(30)
            .all(|row| row.iter().all(|byte| *byte == 0))
    );
}
