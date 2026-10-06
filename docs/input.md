# Input and focus

Prepare a view before forwarding input so hits and caret geometry are available.
Retain the view between frames. Translate native keys and modifiers into `Key`
and `Modifiers`; send committed text separately so it is inserted once.

```rust
use halley_ui::{InputEvent, Key, Modifiers, Rect, TextInput, TextSystem, Theme, UiView};
let mut view = UiView::new("dialog").content(
    TextInput::new("name", "").accessible_label("Cluster name"),
);
let mut text = TextSystem::new();
let bounds = Rect::new(0.0, 0.0, 400.0, 160.0);
let theme = Theme::default();
view.prepare(bounds, &theme, &mut text).unwrap();
view.handle_event(InputEvent::KeyDown { key: Key::Tab, modifiers: Modifiers::default() });
view.handle_event(InputEvent::Text("Research".into()));
assert_eq!(view.text_value("name"), Some("Research"));
if view.needs_prepare() { view.prepare(bounds, &theme, &mut text).unwrap(); }
let ime = view.ime_state();
```

Dispatch `Activate`, `TextChanged`, `Submit`, and `Cancel` to application logic.
`FocusChanged` identifies a local widget key; Tab navigation wraps within the
view and skips disabled controls. Pointer movement sets hover, pointer down
sets focus and capture, and pointer up activates an enabled button only when
released inside it. Send `PointerCancel` if native capture is lost.

## Clipboard

`ClipboardWrite(text)` asks the host to copy the selected text.
`ClipboardRead { request }` asks for a paste. Retrieve text asynchronously using
your platform clipboard, then send `InputEvent::Paste { request, text }` back to
the same view. The request is accepted only while its editor, focus, and revision
still match. Keep the request ID intact; avoid inserting text directly as a
clipboard reply. Clipboard ownership and Wayland selection remain host-owned.

## Input methods

Forward committed text, temporary `Preedit`, and `DeleteSurrounding` events.
During preedit, the IME owns editing navigation. Forward Tab/focus changes and
Escape cancellation as appropriate for your platform. The library does not
speak the Wayland text-input or input-method protocols.

After each relevant preparation, apply `ime_state` to your native input-method
integration: enable it for the named editor, publish the committed surrounding
text and UTF-8 selection offsets, and position candidate windows at `caret`.
Use `ime_state_in` with the same translated snapshot used for drawing and hits.
The host converts these coordinates to its native surface/DPI convention.

## Visibility and transforms

Send `WindowFocus(false)` when the window loses focus or the overlay is hidden;
it cancels capture, composition, and pending paste. Send `WindowFocus(true)`
when the view becomes eligible for input again. Gate forwarding for invisible
views even if the underlying native window remains focused.

If the host moves a prepared view using `translated`, forward pointer events
with `handle_event_in(&translated, event)`. Stale snapshots from an older
content structure are rejected. Prepare again after structural changes before
routing input. The host requests a repaint when `needs_prepare()` becomes true;
idle pointer movement over unchanged targets preserves the cached layout.
