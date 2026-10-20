# Slider

A horizontal numeric slider with retained values, pointer dragging, keyboard input,
theme styles, and accessibility semantics.

```rust
use halley_ui::{InputEvent, Key, Modifiers, Rect, Slider, TextSystem, Theme, UiEvent, UiView};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut view = UiView::new("audio").content(
    Slider::new("volume", 0.0, 100.0).label("Volume").value(50.0).step(5.0),
);
view.prepare(Rect::new(0.0, 0.0, 400.0, 160.0), &Theme::default(), &mut TextSystem::new())?;
view.focus("volume")?;
assert_eq!(view.handle_event(InputEvent::KeyDown {
    key: Key::Right, modifiers: Modifiers::default(),
}), vec![UiEvent::SliderChanged { key: "volume".into(), value: 55.0 }]);
assert_eq!(view.slider_value("volume"), Some(55.0));
view.set_slider_value("volume", 73.0)?; // Silent update; snaps to 75.
assert_eq!(view.slider_value("volume"), Some(75.0));
# Ok(())
# }
```

## Range and retained state

Ranges use `f64`. Minimum, maximum, initial value, and optional step must be finite;
minimum must be smaller than maximum, their difference must be finite, and a step
must be positive. Preparation rejects invalid specifications. Values outside the
range are clamped. A step snaps values to the nearest point anchored at the
minimum; both endpoints are reachable even when the step does not divide the
range. Without a step, pointer and programmatic values are continuous. A grid
finer than floating-point precision is effectively continuous.

The `value` builder seeds state once per key. Rebuilding content preserves that
value; changing the range or step clamps and snaps it silently. Removing the
slider discards its state when the view synchronizes. Read with `slider_value`;
`set_slider_value` accepts finite values, returns whether the value changed, and
emits no event. Disabled sliders accept programmatic updates. Prepare again before
drawing. `UiStack` provides the same setter with a `LayerId`, including inactive
layers; read through `stack.view(&layer)`.

User changes emit `SliderChanged { key, value }` only when the value changes.
They do not emit `Activate`, so adjusting a dialog's slider does not close it.
The toolkit leaves units, value formatting, persistence, and business validation
to the application. This initial control is horizontal and has one thumb.

## Input

Pressing the track sets the value; dragging continues outside the control and
clamps to the endpoints. Grabbing the thumb preserves the grab offset. Release
applies its final position and ends capture. Label/padding clicks only focus the
control. Pointer cancel, Escape, focus changes, window blur, changed content, and
a changed programmatic value cancel capture. Cancellation keeps the latest value.
New overlays cancel the underlying layer's capture. Wheel input continues to
scroll enclosing viewports without changing the slider.

A slider has one Tab stop and reveals itself in enclosing scroll containers.
Right/Up increase and Left/Down decrease by one step; a continuous slider uses
1% of its range. PageUp/PageDown use 10% of the range or one step, whichever is
larger, then snap. Home/End select the endpoints. Control/Alt combinations and
Space/Enter leave the value unchanged. Disabled sliders cannot receive input or
focus. Translate presentation snapshots with `PreparedView::translated` and use
`handle_event_in` as for other controls.

## Styling and accessibility

`Theme::styles.slider` and the widget's `style` support normal, hover, press,
focus, error, and disabled rules. `fill` colors the unfilled track, `selection`
the filled track, `text` the label and thumb, and `border`/`border_width` the
track and thumb border. Track/thumb rounding follows their geometry; the style's
radius applies to the outer focus outline. Numeric position does not apply a
`selected` rule. A label appears above the track and ellipsizes when constrained.

`Theme::forms` supplies `slider_width` (intrinsic content width),
`slider_track_height`, and `slider_thumb_size`, plus shared form padding and
indicator/label gap. Metrics must be finite and nonnegative; thumb and track sizes
must be positive, width must exceed thumb size, and track height cannot exceed
thumb size. Explicit layout dimensions override intrinsic size. Small layouts
shrink the thumb; a track with no thumb travel has no pointer adjustment.

The prepared semantic node exposes `NumericValue { value, range }`. With
accessibility enabled, AccessKit exposes a horizontal Slider, label, numeric
value, min/max, step/jump, disabled/invalid state, and focus. Use a visible `label`
or `accessible_label` for a meaningful name. SetValue with numeric data and
Increment/Decrement requests produce the same changes as user input. Native
screen-reader and input integration still need validation in each host.

## API additions

Complete `FormMetrics` literals need the three slider metrics; complete
`ComponentStyles` literals need `slider`, and `SemanticNode` gains `numeric_value`.
Exhaustive matches on `UiEvent`, `ControlKind`, `SemanticRole`, `LayoutError`,
`InteractionError`, and `AccessibleAction` must handle new variants. Rendering
continues to use existing card/text paint items. Prefer `..Default::default()`
for theme configuration.
