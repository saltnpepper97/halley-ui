# Row and Column

`Row` arranges children horizontally; `Column` arranges them vertically. Use
`gap` for spacing and `align(ui::Align)` for cross-axis alignment. Each widget
supports `padding`, `padding_xy`, `width`, `height`, `max_width`, `max_height`,
and `grow`. Values are in the same UI units as the supplied preparation bounds.

```rust
use halley_ui::{Button, Column, Label, Row};
let content = Column::new("content").gap(12.0)
    .child(Label::new("title", "Create cluster"))
    .child(Row::new("actions").gap(8.0)
        .child(Button::new("cancel", "Cancel"))
        .child(Button::new("create", "Create")));
```

Taffy computes sizes and positions. Leaf measurements come from shared text
services or the host's measurement callback. Use `UiView::anchor` and `margin`
to position the root inside the available rectangle; the default anchor is
`TopCenter`. Drawing does not apply automatic DPI scaling.

Use `Scroll` to make overflowing content navigable. Ordinary containers clip
their descendants to their content rectangle. Keys are unique across the entire
view, and keyboard focus follows enabled controls in tree order.
