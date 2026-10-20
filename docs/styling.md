# Themes and component styles

`Theme` supplies base colors, the body font, spacing defaults, typography roles,
and component rules for buttons, text inputs, lists, and list options. The default
theme preserves the existing appearance. Styling changes paint and layout data;
it does not own application state, native events, or the window's theme settings.

## Partial overrides

Every property in `Style` is optional. Unset properties inherit earlier rules.
Use `Theme::styles` for shared component rules and a widget's `style` builder for
local overrides. `List::item_style` supplies an intermediate layer for all of
that list's options. An individual option's rules win over the list-wide rules
for the same state and property.

```rust
use halley_ui::{Button, Color, Outline, Style, StyleSheet, Theme};

let mut theme = Theme::default();
theme.styles.button.normal.fill = Some(Color::rgb8(35, 65, 75));
theme.styles.button.hovered.fill = Some(Color::rgb8(45, 85, 95));
theme.styles.button.focused.outline = Some(Outline {
    color: Color::rgb8(100, 220, 190),
    width: 2.0,
});

let destructive = Button::new("delete", "Delete").style(StyleSheet {
    normal: Style { text: Some(Color::rgb8(245, 130, 130)), ..Default::default() },
    // Everything else still comes from the theme, including hover and focus.
    ..Default::default()
});
```

`Style` controls fill, text, border color/width, rounding, and an outline. Text
inputs additionally use placeholder, selection, caret, and preedit colors.
An outline shares the control's bounds and resolved radius; an outline width of
zero explicitly disables it. Setting a field to `None` means inheritance, so use
a transparent color or zero width when you want to hide something.

## State precedence

Rules apply in this order:

1. Normal
2. Selected
3. Hovered
4. Pressed
5. Focused
6. Error
7. Disabled

These are independent flags. A selected option can also be hovered and focused:
the selection fill, hover text, and focus outline can all remain in effect.
Selected rules apply to list options; text selection has its own highlight color.
Only a property set by a later rule replaces that property. For each state,
component defaults apply first, followed by the theme rule, list-wide item rule,
and individual widget rule. A local normal fill does not replace a theme hover
fill; set the local hovered rule to override that state.

Disabled controls ignore hover, press, and focus flags. Their disabled rules
apply after normal/error rules, and input remains disabled regardless of colors.
With no custom disabled rule, buttons and inputs retain the existing half-alpha
fill behavior. Default selected list options keep their selection fill on hover;
an explicit hovered fill can override it.

`invalid(true)` enables a control's error style and exposes its invalid state to
AccessKit. It does not prevent editing or activation. The default error border
uses `Theme::error`. Business validation and associated error messages remain
application-owned. Disabled options still cannot become selected.

## Metrics and typography

`StyleSheet::font` and `padding` are independent of state. Hover/focus rules can
change paint without moving hit regions or causing hover-induced layout changes.
The resolved font and padding are shared by layout, text fitting, painting,
pointer selection, scrolling, and caret/IME geometry.

```rust
use halley_ui::{Font, Insets, Label, TextInput, TextRole, Theme};

let mut theme = Theme::default();
theme.spacing.button_padding = Insets::xy(16.0, 10.0);
theme.spacing.container_gap = 8.0;
theme.spacing.list_gap = 4.0;
theme.styles.text_input.font = Some(Font { family: "sans-serif".into(), size: 18 });
theme.typography.title = Some(Font { family: "sans-serif".into(), size: 26 });
theme.typography.caption = Some(Font { family: "sans-serif".into(), size: 13 });

let title = Label::new("title", "Create cluster").typography(TextRole::Title);
let input = TextInput::new("name", "").padding(0.0); // Explicit zero wins over theme padding.
```

`Theme::font` is the body font. Title/caption roles inherit the surrounding font
when unset. Custom content inside a button or option inherits its resolved text
color and font, including state text colors. A label's explicit `color` or `font`
builder wins; a requested typography role overrides the inherited font before
an explicit label font applies.

Metric precedence is component spacing defaults, theme component metrics,
list-wide item metrics, widget metrics, then explicit widget `padding` builders.
Explicit `gap` builders similarly override the theme's container/list gap.
`Spacing::small`, `medium`, and `large` are convenience values for application
composition. Builder floats are captured values: rebuild content if you change
a convenience token already passed to a builder. Component metric defaults are
resolved again automatically when a changed theme is prepared.

## Updating a retained view

Request a repaint and call `prepare` with the changed theme. Preparation compares
the complete theme, including component rules and metrics, against its cached
theme. A changed theme recomputes layout and painting while retaining editor
contents, text selection, list selection, and widget focus. `needs_prepare`
reports changes owned by the view; it cannot observe mutations to an external
theme before you pass that theme to `prepare`.

Rebuild content with the same keys to change widget-local styles or validation
state. Then prepare before drawing, routing geometry-dependent input, or
publishing accessibility updates. Finite colors and nonnegative, finite style
dimensions are required; inactive state rules are validated too.

## API migration

Construct themes with `Theme { ... , ..Default::default() }` or mutate
`Theme::default()`. Existing base fields (`font`, `text`, `subtext`, `card`,
`button`, `button_hover`, `button_pressed`) still provide component defaults.
Complete struct literals need the new `error`, `spacing`, `typography`, and
`styles` fields. `SemanticNode` now includes `invalid`.

`PreparedView::button_visuals` has been removed. Forward pointer/keyboard input
through the retained `UiView`, then prepare and draw its result. This resolves
all component/state overrides together, including custom child text, instead of
replacing button fills on an already prepared snapshot.
