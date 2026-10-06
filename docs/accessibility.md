# Accessibility

Enable `accessibility` for an AccessKit tree, or `accessibility-unix` for the
Linux AT-SPI bridge as well. Neither feature adds Smithay or Wayland.

```toml
halley-ui = { version = "0.1", features = ["accessibility-unix"] }
```

A prepared view exposes roles, accessible names, full label/input values,
disabled/read-only state, stable widget identities, text selection, scroll
ranges, bounds, and keyboard focus. Text inputs contain `TextRun` children so
native text interfaces can inspect and edit their contents. Meaningful images
need labels; unlabelled images are decorative.

## Host integration

Keep one `AccessibilityTree` or `UnixBridge` per native top-level view. Share
actual native window focus, not simply whether a widget has an internal focus.
Publish updates after preparation whenever content, selection, focus, scroll,
geometry, or visibility changes. Adapter callbacks arrive on another thread;
use the supplied wake callback to schedule host/UI-thread work even when no
normal repaint is pending.

```rust,no_run
# #[cfg(target_os = "linux")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use halley_ui::{Rect, TextInput, TextSystem, Theme, UiView};
use halley_ui::accessibility::unix::UnixBridge;
let mut view = UiView::new("dialog").content(
    TextInput::new("name", "Work").accessible_label("Cluster name"),
);
let mut text = TextSystem::new();
let theme = Theme::default();
let bounds = Rect::new(0.0, 0.0, 400.0, 160.0);
let prepared = view.prepare(bounds, &theme, &mut text)?.clone();
let (wake_sender, wake_receiver) = std::sync::mpsc::channel();
let mut bridge = UnixBridge::new(&prepared, "Create cluster", move || {
    let _ = wake_sender.send(());
});
bridge.update(&prepared, "Create cluster", true);
// In the real host, the wake source must wake its native event loop.
let _ = wake_receiver.try_recv();
for action in bridge.actions() {
    let app_events = view.handle_accessibility(action);
    // Dispatch these application/clipboard events using the same path as input.
    let _ = app_events;
}
let prepared = view.prepare(bounds, &theme, &mut text)?;
bridge.update(prepared, "Create cluster", true);
# Ok(())
# }
# #[cfg(not(target_os = "linux"))]
# fn main() {}
```

`AccessibilityTree::action` validates request ownership and translates supported
requests into `AccessibleAction`. Apply them through `handle_accessibility` on
the UI thread, dispatch returned application events, then prepare and publish
again. Revealing a control preserves keyboard focus. The host should activate
its native window when assistive focus navigation requires it and publish the
resulting real focus state. Other platforms can connect the exported tree to
their own AccessKit adapters.

## Verification and limits

Tests validate the tree with AccessKit's consumer and perform real AT-SPI reads,
edits, button activation, and focus requests on a private bus. To reproduce on
a Linux system with the AT-SPI launcher/registry installed:

```sh
env GSETTINGS_BACKEND=memory dbus-run-session -- \
  env HALLEY_UI_PRIVATE_ATSPI_TEST=1 \
  cargo test --all-features --test accessibility_unix -- --ignored --test-threads=1
```

The test starts its own launcher/registry and refuses to run without the private
bus marker and memory-only settings. It does not enable accessibility in the
user's real desktop settings. The default paths target Arch's AT-SPI package.

A screen-reader user journey, speech announcements, native surface integration,
real IME composition, and high-DPI caret/magnifier behavior still need checks in
each consuming app. Version 0.1.0 supplies the bridge and basic semantics; it does
not claim those applications are fully accessible automatically. AccessKit's
UTF-8 character-length field is limited to 255 bytes, so exceptionally longer
grapheme clusters use scalar chunks in its text representation; editor selection
still snaps to complete graphemes.
