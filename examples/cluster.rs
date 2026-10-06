//! Headless rendering plus scripted editing; the host owns any actual window/event loop.
use halley_ui::{
    Anchor, Button, Card, Color, Column, Font, InputEvent, Key, Label, Modifiers, Rect, Row,
    Scroll, TextInput, TextSystem, Theme, UiView, software::PixelBuffer,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/halley-ui-cluster.png".into());
    let theme = Theme {
        font: Font {
            family: "monospace".into(),
            size: 18,
        },
        ..Default::default()
    };
    let mut view = UiView::new("cluster").anchor(Anchor::Center).content(
        Card::new("card").padding(16.0).child(
            Column::new("content")
                .gap(12.0)
                .child(Label::new("title", "Create cluster"))
                .child(
                    TextInput::new("name", "")
                        .width(320.0)
                        .placeholder("Enter a name")
                        .accessible_label("Cluster name"),
                )
                .child(Label::new("recent-label", "Recent clusters").color(theme.subtext))
                .child(
                    Scroll::new("recent").width(320.0).height(84.0).child(
                        Column::new("recent-items")
                            .gap(4.0)
                            .child(Button::new("work", "Work").height(36.0))
                            .child(Button::new("personal", "Personal").height(36.0))
                            .child(Button::new("research", "Research").height(36.0)),
                    ),
                )
                .child(
                    Row::new("actions")
                        .gap(8.0)
                        .child(Button::new("cancel", "Cancel"))
                        .child(Button::new("create", "Create")),
                ),
        ),
    );
    let mut text = TextSystem::new();
    let bounds = Rect::new(0.0, 0.0, 480.0, 380.0);
    view.prepare(bounds, &theme, &mut text)?;
    view.handle_event(InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers::default(),
    });
    view.handle_event(InputEvent::Text("Research".into()));
    let prepared = view.prepare(bounds, &theme, &mut text)?;
    let mut pixels = PixelBuffer::new(480, 380)?;
    let mut surface = pixels.surface();
    surface.clear(Color::rgb8(20, 25, 31));
    surface.draw(prepared, &mut text, 1.0);
    image::save_buffer(output, pixels.pixels(), 480, 380, image::ColorType::Rgba8)?;
    Ok(())
}
