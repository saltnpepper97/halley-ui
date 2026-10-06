//! Render a component tree without a compositor, display server, or GPU.
use halley_ui::{
    ActionId, Anchor, Button, Card, Column, Image, Label, Rect, Row, TextSystem, Theme, UiView,
};
use halley_ui::{assets::ImageData, software::PixelBuffer, ui::Align};
use std::{path::Path, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let icon = Arc::new(ImageData::from_svg(br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M4 12l5 5L20 6" fill="none" stroke="#d65d26" stroke-width="3"/></svg>"##, 24)?);
    let mut view = UiView::new("configuration-notice")
        .anchor(Anchor::Center)
        .content(
            Card::new("card").padding(16.0).child(
                Column::new("content")
                    .gap(12.0)
                    .child(
                        Row::new("message-row")
                            .gap(10.0)
                            .align(Align::Center)
                            .child(Image::new("icon", icon).width(24.0).height(24.0))
                            .child(Label::new("message", "Configuration loaded")),
                    )
                    .child(
                        Button::new("dismiss", "Got it").action(ActionId::new("dismiss-notice")),
                    ),
            ),
        );
    let mut text = TextSystem::new();
    let mut theme = Theme::default();
    theme.font.size = 18;
    let prepared = view.prepare(Rect::new(0.0, 0.0, 480.0, 160.0), &theme, &mut text)?;
    let mut pixels = PixelBuffer::new(480, 160)?;
    pixels.surface().draw(prepared, &mut text, 1.0);
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/halley-ui-notice.png".into());
    image::save_buffer(
        Path::new(&output),
        pixels.pixels(),
        480,
        160,
        image::ColorType::Rgba8,
    )?;
    println!("Rendered notice to {output}");
    Ok(())
}
