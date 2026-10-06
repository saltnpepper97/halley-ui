# Scroll

`Scroll` is a clipped viewport with column layout. Supply a bounded width and
height and put larger content inside it. It supports both axes, wheel movement,
and revealing a focused control.

```rust
use halley_ui::{Button, Column, Scroll};
let list = Scroll::new("viewport").width(240.0).height(96.0).child(
    Column::new("items").gap(4.0)
        .child(Button::new("one", "First").height(40.0))
        .child(Button::new("two", "Second").height(40.0))
        .child(Button::new("three", "Third").height(40.0)),
);
```

`InputEvent::Wheel` uses pixel deltas: positive movement scrolls towards the end
of content. The host converts line-based wheel events to suitable UI pixels.
Offsets clamp to the content extent. Unconsumed movement bubbles to an ancestor
viewport; non-finite deltas are ignored.

Painting, hit regions, and semantic bounds use the same scroll transform and
clip. Tab can reach enabled offscreen controls and reveal them before the next
frame. Accessibility `ScrollIntoView` reveals a control without changing focus.

`PreparedView::scrolls` exposes viewport, extent, offset, and ancestry for host
integration. Content changes clamp offsets at the next preparation. Version
0.1.0 has no scrollbars, kinetic animation, or list virtualization.
