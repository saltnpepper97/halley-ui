# Halley UI

A small Rust UI toolkit for the Halley ecosystem:
[Taffy](https://github.com/DioxusLabs/taffy) layout, components, Unicode text
editing, focus, scrolling, shared text rendering, and direct software drawing.
The toolkit is independent of Smithay and Wayland.

## Quick start

```toml
[dependencies]
halley-ui = "0.1"
```

```rust
use halley_ui::{ActionId, Button, Card, Column, Label, Rect, TextSystem, Theme, UiView};
use halley_ui::software::PixelBuffer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut notice = UiView::new("notice").content(
        Card::new("card").padding(16.0).child(
            Column::new("content").gap(12.0)
                .child(Label::new("message", "Configuration loaded"))
                .child(Button::new("dismiss", "Got it")
                    .action(ActionId::new("dismiss-notice"))),
        ),
    );
    let mut text = TextSystem::new();
    let prepared = notice.prepare(Rect::new(0.0, 0.0, 480.0, 160.0),
        &Theme::default(), &mut text)?;
    let mut pixels = PixelBuffer::new(480, 160)?;
    pixels.surface().draw(prepared, &mut text, 1.0);
    Ok(())
}
```

Run `cargo run --example notice -- /tmp/halley-ui-notice.png` to render the
included notification. `cargo run --example cluster -- /tmp/halley-ui-cluster.png`
renders an input-and-scroll example after a short scripted edit.

## Using it in an app

Retain `UiView` and `TextSystem` between frames. Each widget needs a unique key
within its view. Layout and shaped text are cached. Use `set_content` for
structural changes and `set_text_value` to replace a retained editor's value.

Forward host events to `handle_event`; dispatch its `UiEvent` results to your
application, clipboard, and input method. Prepare again when `needs_prepare()`
is true. For a translated snapshot, use `handle_event_in` and `ime_state_in`
with that same snapshot so paint, hits, and IME geometry agree.

- **Components:** cards, containers, rows, columns, labels, buttons, images,
  single-line text inputs, and clipped scroll containers.
- **Drawing:** use an owned `PixelBuffer` or borrow existing RGBA/BGRA pixels
  with `Surface::new`. Buffers use premultiplied alpha and support padded stride.
- **Host renderers:** consume `PreparedView::items` or supply existing glyph
  measurements through `prepare_with_color_measure`. The host owns its GPU
  textures, damage tracking, Wayland surfaces, and buffer lifetime.
- **Accessibility:** `accessibility` exports an AccessKit tree;
  `accessibility-unix` also supplies a Linux AT-SPI bridge. The host must connect
  wakeups, actions, updates, and actual window focus.

## Documentation

[Component guides and integration documentation](https://github.com/saltnpepper97/halley-ui/blob/main/docs/README.md)
cover each widget, input, drawing, and accessibility. Rust API documentation is
available through `cargo doc --all-features --no-deps`.

Version 0.1.0 is an initial API. Multiline editing, undo history, virtualized
lists, live window previews, and advanced effects are outside its current scope.
Keyboard left/right movement follows logical grapheme order; shaped cursor and
selection drawing supports bidirectional text. Interactive screen-reader and
real input-method journeys still need validation in each consuming application.

Run `cargo test --all-features` and
`cargo clippy --all-features --all-targets -- -D warnings`.

GPL-3.0-only. Shared text/rendering code was extracted from Halley under its
existing license. Historical migration evidence and the deferred GPU prototype
are retained in this repository, outside the published package.
