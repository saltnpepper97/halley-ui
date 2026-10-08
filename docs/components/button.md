# Button

`Button::new(key, label)` uses the key as its default `ActionId`. Override it
with `action`. An enabled button supports pointer release inside the same
button, Enter while focused, and Space release after Space press.

```rust
use halley_ui::{ActionId, Button};
let create = Button::new("create", "Create")
    .action(ActionId::new("create-cluster"));
let unavailable = Button::new("unavailable", "Unavailable").disabled(true);
```

Forward pointer and keyboard events through `UiView::handle_event`. The toolkit
returns `UiEvent::Activate`; the host performs the business action. Tab and
Shift-Tab navigate enabled controls. Focus has a visible outline; hover and
press use the theme's button colors.

Disabled buttons are omitted from focus navigation and do not activate.
`accessible_label` overrides the visible label as an accessible name when
needed. `PreparedView::hit` remains available for hosts that already manage
pointer dispatch; use one activation policy to avoid dispatching twice.

Use `Button::new("open", "").accessible_label("Open item").child(row)` for a
button containing an image, title, subtitle, or shortcut hint. Its children use
[Taffy](https://github.com/DioxusLabs/taffy) layout, and the full button remains
the activation target.
