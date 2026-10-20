#![doc = include_str!("../docs/styling.md")]
//! Component themes, state rules, and partial widget overrides.
pub use super::CardStyle;
use super::{Kind, LayoutError, Widget};
use crate::{Color, Font, Insets};
use std::collections::HashMap;

/// Visual flags can coexist: a selected option can also be hovered and focused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub selected: bool,
    pub disabled: bool,
    pub invalid: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub color: Color,
    /// Zero disables the outline. The outline shares the control's bounds and radius.
    pub width: f32,
}

/// Every unset property inherits the earlier rule. These properties never affect layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    pub fill: Option<Color>,
    pub text: Option<Color>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub radius: Option<f32>,
    pub outline: Option<Outline>,
    pub placeholder: Option<Color>,
    pub selection: Option<Color>,
    pub caret: Option<Color>,
    pub preedit: Option<Color>,
}

/// Metrics are state-independent; state rules only change painting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleSheet {
    pub font: Option<Font>,
    pub padding: Option<Insets>,
    pub normal: Style,
    pub selected: Style,
    pub hovered: Style,
    pub pressed: Style,
    pub focused: Style,
    pub error: Style,
    pub disabled: Style,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentStyles {
    pub button: StyleSheet,
    pub text_input: StyleSheet,
    pub list: StyleSheet,
    pub list_item: StyleSheet,
    pub menu: StyleSheet,
    pub menu_item: StyleSheet,
    pub slider: StyleSheet,
    pub checkbox: StyleSheet,
    pub switch: StyleSheet,
    pub radio_group: StyleSheet,
    pub radio_option: StyleSheet,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing {
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub button_padding: Insets,
    pub text_input_padding: Insets,
    pub list_item_padding: Insets,
    pub container_gap: f32,
    pub list_gap: f32,
}
impl Default for Spacing {
    fn default() -> Self {
        Self {
            small: 4.0,
            medium: 8.0,
            large: 16.0,
            button_padding: Insets::xy(12.0, 8.0),
            text_input_padding: Insets::xy(10.0, 8.0),
            list_item_padding: Insets::xy(12.0, 8.0),
            container_gap: 0.0,
            list_gap: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextRole {
    #[default]
    Body,
    Title,
    Caption,
}

/// Unset roles inherit the surrounding font (or `Theme::font` at the root).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Typography {
    pub title: Option<Font>,
    pub caption: Option<Font>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Body font; component and role fonts can override it.
    pub font: Font,
    pub text: Color,
    pub subtext: Color,
    pub card: CardStyle,
    pub button: CardStyle,
    pub button_hover: Color,
    pub button_pressed: Color,
    pub error: Color,
    pub spacing: Spacing,
    pub typography: Typography,
    pub styles: ComponentStyles,
    pub overlays: super::overlays::OverlayTheme,
    pub forms: super::forms::FormMetrics,
}
impl Default for Theme {
    fn default() -> Self {
        let border = Color::rgb8(214, 93, 38);
        Self {
            font: Font::default(),
            text: Color::rgb8(240, 245, 250),
            subtext: Color::rgb8(190, 195, 200),
            card: CardStyle {
                fill: Color::rgb8(38, 46, 56),
                border,
                border_width: 3.0,
                radius: 8.0,
            },
            button: CardStyle {
                fill: Color::rgb8(58, 66, 76),
                border,
                border_width: 0.0,
                radius: 8.0,
            },
            button_hover: Color::rgb8(78, 86, 96),
            button_pressed: Color::rgb8(98, 106, 116),
            error: Color::rgb8(220, 70, 70),
            spacing: Spacing::default(),
            typography: Typography::default(),
            styles: ComponentStyles::default(),
            overlays: super::overlays::OverlayTheme::default(),
            forms: super::forms::FormMetrics::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ResolvedStyle {
    pub surface: CardStyle,
    pub text: Color,
    pub font: Font,
    pub padding: Insets,
    pub outline: Outline,
    pub placeholder: Color,
    pub selection: Color,
    pub caret: Color,
    pub preedit: Color,
    pub state: WidgetState,
}
impl Style {
    fn apply(self, resolved: &mut ResolvedStyle) {
        macro_rules! set { ($($field:ident),*) => { $(if let Some(value) = self.$field { resolved.$field = value; })* }; }
        macro_rules! surface { ($($field:ident),*) => { $(if let Some(value) = self.$field { resolved.surface.$field = value; })* }; }
        set!(text, outline, placeholder, selection, caret, preedit);
        surface!(fill, border, border_width, radius);
    }
    fn valid(self) -> bool {
        [
            self.fill,
            self.text,
            self.border,
            self.placeholder,
            self.selection,
            self.caret,
            self.preedit,
        ]
        .into_iter()
        .flatten()
        .all(color_valid)
            && [self.border_width, self.radius]
                .into_iter()
                .flatten()
                .all(nonnegative)
            && self
                .outline
                .is_none_or(|o| color_valid(o.color) && nonnegative(o.width))
    }
}
impl StyleSheet {
    fn valid(&self) -> bool {
        self.padding.is_none_or(insets_valid)
            && [
                self.normal,
                self.selected,
                self.hovered,
                self.pressed,
                self.focused,
                self.error,
                self.disabled,
            ]
            .into_iter()
            .all(Style::valid)
    }
}
fn nonnegative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}
fn color_valid(color: Color) -> bool {
    [color.r, color.g, color.b, color.a]
        .into_iter()
        .all(f32::is_finite)
}
fn insets_valid(value: Insets) -> bool {
    [value.left, value.right, value.top, value.bottom]
        .into_iter()
        .all(nonnegative)
}
fn surface_valid(style: CardStyle) -> bool {
    color_valid(style.fill)
        && color_valid(style.border)
        && nonnegative(style.border_width)
        && nonnegative(style.radius)
}

impl Theme {
    pub(super) fn validate(&self) -> Result<(), LayoutError> {
        let spacing = self.spacing;
        if ![
            self.text,
            self.subtext,
            self.button_hover,
            self.button_pressed,
            self.error,
        ]
        .into_iter()
        .all(color_valid)
            || !surface_valid(self.card)
            || !surface_valid(self.button)
            || ![
                spacing.small,
                spacing.medium,
                spacing.large,
                spacing.container_gap,
                spacing.list_gap,
            ]
            .into_iter()
            .all(nonnegative)
            || ![
                spacing.button_padding,
                spacing.text_input_padding,
                spacing.list_item_padding,
            ]
            .into_iter()
            .all(insets_valid)
            || ![
                &self.styles.button,
                &self.styles.text_input,
                &self.styles.list,
                &self.styles.list_item,
                &self.styles.menu,
                &self.styles.menu_item,
                &self.styles.slider,
                &self.styles.checkbox,
                &self.styles.switch,
                &self.styles.radio_group,
                &self.styles.radio_option,
            ]
            .into_iter()
            .all(StyleSheet::valid)
        {
            return Err(LayoutError::InvalidStyle("theme".into()));
        }
        self.overlays.validate()?;
        self.forms.validate()
    }
}

pub(super) fn resolve_tree(
    root: &Widget,
    theme: &Theme,
    state: &crate::input::State,
) -> Result<HashMap<String, ResolvedStyle>, LayoutError> {
    fn visit(
        widget: &Widget,
        theme: &Theme,
        state: &crate::input::State,
        parent: Option<&ResolvedStyle>,
        list: Option<&Widget>,
        out: &mut HashMap<String, ResolvedStyle>,
    ) -> Result<(), LayoutError> {
        let is_button = matches!(widget.kind, Kind::Button { .. });
        let is_input = matches!(widget.kind, Kind::TextInput { .. });
        let is_list = matches!(widget.kind, Kind::List { .. });
        let is_item = matches!(widget.kind, Kind::ListItem { .. });
        let is_slider = matches!(widget.kind, Kind::Slider { .. });
        let is_choice = super::forms::is_choice(&widget.kind);
        let is_radio = matches!(widget.kind, Kind::RadioOption { .. });
        let disabled = match widget.kind {
            Kind::Button { disabled, .. }
            | Kind::TextInput { disabled, .. }
            | Kind::List { disabled, .. }
            | Kind::ListItem { disabled, .. }
            | Kind::Slider { disabled, .. }
            | Kind::Checkbox { disabled, .. }
            | Kind::Switch { disabled, .. }
            | Kind::RadioGroup { disabled, .. }
            | Kind::RadioOption { disabled, .. } => disabled,
            _ => false,
        } || parent.is_some_and(|p| p.state.disabled);
        let selected = if matches!(widget.kind, Kind::Checkbox { .. } | Kind::Switch { .. }) {
            state.checked.get(&widget.key).copied().unwrap_or(false)
        } else if is_radio {
            list.is_some_and(|l| {
                state.radios.get(&l.key).and_then(|s| s.as_deref()) == Some(widget.key.as_str())
            })
        } else {
            is_item
                && list.is_some_and(|l| {
                    state.lists.get(&l.key).and_then(|s| s.as_deref()) == Some(widget.key.as_str())
                })
        };
        let focused = !disabled
            && state.window_focused
            && (state.focus.as_deref() == Some(widget.key.as_str())
                || (selected
                    && list.is_some_and(|l| state.focus.as_deref() == Some(l.key.as_str()))));
        let flags = WidgetState {
            hovered: !disabled && state.hover.as_deref() == Some(widget.key.as_str()),
            pressed: !disabled && state.pressed.as_deref() == Some(widget.key.as_str()),
            focused,
            selected,
            disabled,
            invalid: widget.invalid,
        };
        let transparent = Color::rgba(0.0, 0.0, 0.0, 0.0);
        let mut surface = match widget.kind {
            Kind::Card { style } => style.unwrap_or(theme.card),
            Kind::Button { .. }
            | Kind::TextInput { .. }
            | Kind::Slider { .. }
            | Kind::Checkbox { .. }
            | Kind::Switch { .. }
            | Kind::RadioOption { .. } => theme.button,
            _ => CardStyle {
                fill: transparent,
                border: transparent,
                border_width: 0.0,
                radius: if is_list || is_item {
                    theme.button.radius
                } else {
                    0.0
                },
            },
        };
        if is_choice {
            surface.border_width = 1.0;
            surface.radius = if matches!(widget.kind, Kind::Checkbox { .. }) {
                theme.forms.indicator_size * 0.2
            } else {
                theme.forms.indicator_size * 0.5
            };
        }
        let mut resolved = ResolvedStyle {
            surface,
            text: parent.map_or(theme.text, |p| p.text),
            font: parent.map_or_else(|| theme.font.clone(), |p| p.font.clone()),
            padding: if is_button {
                theme.spacing.button_padding
            } else if is_input {
                theme.spacing.text_input_padding
            } else if is_choice || is_slider {
                theme.forms.padding
            } else if is_item {
                theme.spacing.list_item_padding
            } else {
                Insets::default()
            },
            outline: Outline {
                color: theme.card.border,
                width: 0.0,
            },
            placeholder: theme.subtext,
            selection: Color::rgba(0.3, 0.5, 0.8, 0.5),
            caret: theme.text,
            preedit: theme.text,
            state: flags,
        };
        if is_input {
            resolved.surface.border_width = 1.0;
        }
        let component = if is_button {
            Some(&theme.styles.button)
        } else if is_input {
            Some(&theme.styles.text_input)
        } else if is_list {
            Some(&theme.styles.list)
        } else if is_slider {
            Some(&theme.styles.slider)
        } else if matches!(widget.kind, Kind::Checkbox { .. }) {
            Some(&theme.styles.checkbox)
        } else if matches!(widget.kind, Kind::Switch { .. }) {
            Some(&theme.styles.switch)
        } else if matches!(widget.kind, Kind::RadioGroup { .. }) {
            Some(&theme.styles.radio_group)
        } else if is_radio {
            Some(&theme.styles.radio_option)
        } else if is_item {
            Some(&theme.styles.list_item)
        } else {
            None
        };
        let layers: Vec<_> = component
            .into_iter()
            .chain(if is_item || is_radio {
                list.and_then(|l| l.item_style.as_deref())
            } else {
                None
            })
            .chain(widget.style.as_deref())
            .collect();
        if !layers.iter().all(|s| s.valid())
            || widget.item_style.as_deref().is_some_and(|s| !s.valid())
        {
            return Err(LayoutError::InvalidStyle(widget.key.clone()));
        }
        for sheet in &layers {
            if let Some(font) = &sheet.font {
                resolved.font = font.clone();
            }
            if let Some(padding) = sheet.padding {
                resolved.padding = padding;
            }
            sheet.normal.apply(&mut resolved);
        }
        if selected {
            resolved.surface.fill = list
                .and_then(|l| match l.kind {
                    Kind::List {
                        selection_color, ..
                    } => selection_color,
                    _ => None,
                })
                .unwrap_or(theme.button_pressed);
            for sheet in &layers {
                sheet.selected.apply(&mut resolved);
            }
        }
        if flags.hovered {
            if is_button || is_choice || is_slider || (is_item && !selected) {
                resolved.surface.fill = theme.button_hover;
            }
            for sheet in &layers {
                sheet.hovered.apply(&mut resolved);
            }
        }
        if flags.pressed {
            if is_button || is_choice || is_slider {
                resolved.surface.fill = theme.button_pressed;
            }
            for sheet in &layers {
                sheet.pressed.apply(&mut resolved);
            }
        }
        if focused {
            if is_button || is_list || is_choice || is_slider {
                resolved.outline.width = 2.0;
            }
            if is_input {
                resolved.surface.border_width = 2.0;
            }
            for sheet in &layers {
                sheet.focused.apply(&mut resolved);
            }
        }
        if flags.invalid {
            resolved.surface.border = theme.error;
            resolved.surface.border_width = resolved.surface.border_width.max(1.0);
            for sheet in &layers {
                sheet.error.apply(&mut resolved);
            }
        }
        if disabled {
            if is_slider {
                resolved.selection.a *= 0.5;
            }
            if is_button || is_input || is_item || is_choice || is_slider {
                resolved.surface.fill.a *= 0.5;
            }
            if is_item || is_choice || is_slider {
                resolved.text = theme.subtext;
            }
            for sheet in &layers {
                sheet.disabled.apply(&mut resolved);
            }
        }
        if let Some(padding) = widget.layout.padding {
            resolved.padding = padding;
        }
        if let Kind::Label {
            font, color, role, ..
        } = &widget.kind
        {
            if let Some(role_font) = match role {
                TextRole::Body => None,
                TextRole::Title => theme.typography.title.as_ref(),
                TextRole::Caption => theme.typography.caption.as_ref(),
            } {
                resolved.font = role_font.clone();
            }
            if let Some(font) = font {
                resolved.font = font.clone();
            }
            if let Some(color) = color {
                resolved.text = *color;
            }
        }
        if !insets_valid(resolved.padding) || !surface_valid(resolved.surface) {
            return Err(LayoutError::InvalidStyle(widget.key.clone()));
        }
        for child in &widget.children {
            visit(
                child,
                theme,
                state,
                Some(&resolved),
                if is_list || matches!(widget.kind, Kind::RadioGroup { .. }) {
                    Some(widget)
                } else {
                    list
                },
                out,
            )?;
        }
        out.insert(widget.key.clone(), resolved);
        Ok(())
    }
    let mut out = HashMap::new();
    visit(root, theme, state, None, None, &mut out)?;
    Ok(out)
}
