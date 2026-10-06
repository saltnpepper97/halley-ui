# Card and Container

`Card` draws a panel using `Theme::card`. `Container` groups children without a
background. Both support `child`, `gap`, `align`, padding, sizing, and `grow`.
Children use column layout; put a `Row` inside for horizontal content.

```rust
use halley_ui::{Card, Column, Label};
let panel = Card::new("panel").padding(16.0).max_width(360.0).child(
    Column::new("content").gap(8.0)
        .child(Label::new("title", "Saved"))
        .child(Label::new("details", "Configuration loaded")),
);
```

Use `style(ui::CardStyle { .. })` to override fill, border, border width, and
radius. Colors are straight-alpha sRGB; drawing premultiplies them once.
Children paint and hit-test inside the ancestor's content clip. A panel's key
must be distinct from all descendant keys.

`accessible_label` can name a meaningful group. Decorative layout groups do not
need labels. Cards do not activate by themselves; use a `Button` for an action.
