use halley_ui::{CardStyle, Color, Font, Insets, Outline, Style, StyleSheet, Theme};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Light,
    Dark,
}
impl Appearance {
    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
    pub fn opposite(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

pub struct Palette {
    pub background: Color,
    pub panel: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub hover: Color,
    pub pressed: Color,
}
impl Appearance {
    pub fn palette(self) -> Palette {
        match self {
            Self::Light => Palette {
                background: Color::rgb8(255, 255, 255),
                panel: Color::rgb8(246, 248, 251),
                border: Color::rgb8(211, 218, 228),
                text: Color::rgb8(27, 35, 48),
                muted: Color::rgb8(89, 102, 121),
                accent: Color::rgb8(37, 99, 210),
                hover: Color::rgb8(225, 233, 246),
                pressed: Color::rgb8(205, 220, 244),
            },
            Self::Dark => Palette {
                background: Color::rgb8(22, 27, 35),
                panel: Color::rgb8(33, 40, 51),
                border: Color::rgb8(68, 80, 99),
                text: Color::rgb8(234, 239, 247),
                muted: Color::rgb8(163, 177, 197),
                accent: Color::rgb8(108, 165, 255),
                hover: Color::rgb8(48, 63, 84),
                pressed: Color::rgb8(62, 85, 117),
            },
        }
    }
    pub fn theme(self, scale: f32) -> Theme {
        let p = self.palette();
        let font = |size: f32| Font {
            family: "sans-serif".into(),
            size: (size * scale).round().max(1.0) as u16,
        };
        let mut t = Theme {
            font: font(16.0),
            text: p.text,
            subtext: p.muted,
            card: CardStyle {
                fill: p.panel,
                border: p.border,
                border_width: scale,
                radius: 10.0 * scale,
            },
            button: CardStyle {
                fill: p.background,
                border: p.border,
                border_width: scale,
                radius: 6.0 * scale,
            },
            button_hover: p.hover,
            button_pressed: p.pressed,
            ..Default::default()
        };
        t.typography.title = Some(font(24.0));
        t.typography.caption = Some(font(13.0));
        t.spacing.button_padding = Insets::xy(14.0 * scale, 9.0 * scale);
        t.spacing.text_input_padding = Insets::xy(12.0 * scale, 9.0 * scale);
        t.forms.padding = Insets::xy(8.0 * scale, 8.0 * scale);
        t.forms.indicator_size = 20.0 * scale;
        t.forms.indicator_gap = 10.0 * scale;
        t.forms.switch_width = 38.0 * scale;
        t.forms.group_gap = 2.0 * scale;
        t.forms.slider_width = 220.0 * scale;
        t.forms.slider_track_height = 6.0 * scale;
        t.forms.slider_thumb_size = 22.0 * scale;
        let focus = Style {
            outline: Some(Outline {
                color: p.accent,
                width: 0.0,
            }),
            ..Default::default()
        };
        t.styles.button.focused = focus;
        t.styles.slider.focused = focus;
        t.styles.slider.normal.selection = Some(p.accent);
        t.styles.text_input.normal.selection = Some(Color { a: 0.3, ..p.accent });
        t.styles.text_input.focused.border = Some(p.accent);
        for sheet in [
            &mut t.styles.checkbox,
            &mut t.styles.switch,
            &mut t.styles.radio_option,
        ] {
            sheet.normal.border = Some(p.border);
            sheet.selected = Style {
                fill: Some(p.accent),
                border: Some(p.accent),
                text: Some(p.text),
                ..Default::default()
            };
            sheet.focused = focus;
        }
        t.overlays.backdrop = Color::rgba(0.0, 0.0, 0.0, 0.4);
        t
    }
    pub fn choice_style(self, active: bool) -> StyleSheet {
        let p = self.palette();
        StyleSheet {
            normal: Style {
                fill: Some(if active { p.pressed } else { p.background }),
                text: Some(p.text),
                border: Some(if active { p.accent } else { p.border }),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
