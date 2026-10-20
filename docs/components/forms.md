# Checkbox, Switch, and RadioGroup

Checkboxes and switches retain boolean values. A radio group retains one option
key, or no selection. These controls share the existing focus, scroll, style,
overlay, and accessibility paths and use renderer-neutral card/text paint items.

```rust
use halley_ui::{
    Checkbox, Column, InputEvent, Key, Modifiers, RadioGroup, RadioOption, Rect,
    Switch, TextSystem, Theme, UiEvent, UiView,
};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let settings = Column::new("settings").gap(12.0)
    .child(Checkbox::new("notifications", "Allow notifications").checked(true))
    .child(Switch::new("network", "Enable network access"))
    .child(RadioGroup::new("power-mode").accessible_label("Power mode")
        .selected("balanced")
        .child(RadioOption::new("performance", "Performance"))
        .child(RadioOption::new("balanced", "Balanced"))
        .child(RadioOption::new("quiet", "Quiet").disabled(true)));
let mut view = UiView::new("preferences").content(settings);
let mut text = TextSystem::new();
view.prepare(Rect::new(0.0, 0.0, 500.0, 350.0), &Theme::default(), &mut text)?;
view.focus("notifications")?;
view.handle_event(InputEvent::KeyDown { key: Key::Space, modifiers: Modifiers::default() });
assert_eq!(view.handle_event(InputEvent::KeyUp { key: Key::Space }), vec![
    UiEvent::CheckedChanged { key: "notifications".into(), checked: false },
]);
assert_eq!(view.checked("notifications"), Some(false));
assert_eq!(view.radio_selected("power-mode"), Some("balanced"));

// Programmatic changes update state silently, rather than simulating user input.
view.set_checked("network", true)?;
view.set_radio_selected("power-mode", Some("quiet"))?;
# Ok(())
# }
```

## Values and events

`checked` and `selected` builders are initial seeds. Rebuilding content with the
same keys changes labels, layout, styles, and disabled state without replacing
retained values. Checkbox and Switch share a boolean value type, so replacing
one with the other under the same key retains that value. Removing a control
discards its value when state synchronizes. Removing a selected radio option
clears the group's selection; reordering options preserves it.

After preparing, use `checked` and `radio_selected` to read state. The setters
`set_checked` and `set_radio_selected` validate targets and return whether the
value changed. They do not emit events. Programmatic updates may target disabled
controls/options, allowing the UI to display saved values even while unavailable.
A disabled selected radio option remains checked. Use `None` to clear a group.
Setters cancel captures associated with changed values, so a pending release
cannot immediately undo the update. Prepare again before drawing changed state.

User input emits `CheckedChanged { key, checked }` or
`RadioChanged { key, selected }` only when the value changes. Radio option keys
are the group's values. Selecting an already checked radio option does nothing;
radio clicks never clear selection. These controls do not emit `Activate`, so
changing a setting inside a dialog does not trigger `close_on_activate`.

Within `UiStack`, the events carry their originating `LayerId` as usual. Read
values through `stack.view(&layer)`. Programmatic updates use the stack's
`set_checked` and `set_radio_selected` with a layer ID, including for inactive
layers; prepare the stack afterward. To modify an overlay definition, use
`update` with surviving widget keys to retain its values.

## Keyboard, pointer, and focus

The entire padded control, including its label, is clickable. Pointer activation
requires a matching press and release inside the control's clip. Space activates
on release, including for radio options; repeated presses before that release
do not toggle repeatedly. Enter leaves values unchanged. Focus changes, Escape,
pointer cancellation, disabling/removing the target, and native window blur cancel
an armed control. Native window focus and input translation remain host-owned.

Checkbox and Switch each occupy one Tab stop. RadioGroup occupies one stop,
entering on its enabled selected option, otherwise the first enabled option.
Tab does not select an unselected option. Tab and Shift+Tab leave the group in
the normal form order. Within a radio group, Up/Left and Down/Right wrap through
enabled options and update focus and selection together. Home/End choose the
first/last enabled option. Focused options reveal themselves through enclosing
scroll containers. Empty and entirely disabled groups have no Tab stop.

`focus("group-key")` resolves to the same enabled entry option. Focusing an
individual radio option moves focus without changing its value. `RadioGroup`
accepts only `RadioOption` children; an option cannot appear outside its group.
Group-level disabling blocks interaction with all options. The initial API uses
single-line labels and does not include mixed-state checkboxes or custom content.

## Styling and accessibility

Component rules are `Theme::styles.checkbox`, `switch`, `radio_group`, and
`radio_option`. A group's `option_style` supplies shared option overrides; local
option rules win. `selected` rules apply to checked/on options. Hover, pressed,
focused, error (`invalid`), and disabled rules compose as for other controls.
Fill, border, and rounding style the indicator; text styles both the label and
mark/thumb. The focus outline surrounds the whole padded control. Group surface
rules paint the group's bounds. Disabled values remain visible and report their
checked state independently of disabled styling.

`Theme::forms` contains indicator size, switch width, indicator/label gap, padding,
and group gap. These metrics are state-independent and feed both measurement and
painting. Switch width must be at least the indicator size; metrics must be finite
and nonnegative. Explicit layout padding and group `gap` builders override theme
defaults. Theme changes preserve values and recompute layout, clips, and painting.

With accessibility enabled, these controls expose CheckBox, Switch, RadioGroup,
and RadioButton roles, labels, checked state, disabled state, and validation state.
The visible label is the default accessible name; `accessible_label` overrides it.
Give each group a meaningful name. AccessKit activation follows the same value
changes and disabled checks as pointer/keyboard input. Screen-reader and native
input integration still need validation in the consuming application.

## API migration

Complete `Theme` literals need `forms`; complete `ComponentStyles` literals need
the four form component sheets. `SemanticNode` gains `checked: Option<bool>`.
Exhaustive matches on `UiEvent`, `ControlKind`, `SemanticRole`, `LayoutError`, and
`InteractionError` must handle the new variants. `PaintItem` and renderer
integration are unchanged. Prefer `..Default::default()` for theme structs.
