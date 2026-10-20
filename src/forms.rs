#![doc = include_str!("../docs/components/forms.md")]
//! Retained boolean controls and single-choice radio groups.
use super::*;
use crate::{Key, UiEvent};
use std::time::Duration;

/// Bounded cache of ordinary checkmark glyphs with transparent text padding removed.
#[derive(Default)]
pub(super) struct Assets {
    marks: HashMap<(Font, [u8; 4]), Arc<ImageData>>,
}
impl Assets {
    fn checkmark(
        &mut self,
        size: f32,
        font: &Font,
        color: Color,
        text: &mut Option<TextSystem>,
    ) -> Option<Arc<ImageData>> {
        let mut font = font.normalized();
        font.size = (size * 0.85).round().clamp(1.0, 256.0) as u16;
        let rgb = color.bytes();
        let rgba = [
            rgb[0],
            rgb[1],
            rgb[2],
            (color.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ];
        if let Some(mark) = self.marks.get(&(font.clone(), rgba)) {
            return Some(mark.clone());
        }
        let raster = text
            .get_or_insert_with(TextSystem::new)
            .raster(&font, "✓", rgb)?;
        let width = raster.width as usize;
        let height = raster.height as usize;
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for (i, pixel) in raster.pixels.chunks_exact(4).enumerate() {
            if pixel[3] != 0 {
                left = left.min(i % width);
                right = right.max(i % width + 1);
                top = top.min(i / width);
                bottom = bottom.max(i / width + 1);
            }
        }
        if right <= left || bottom <= top {
            return None;
        }
        let mut pixels = Vec::with_capacity((right - left) * (bottom - top) * 4);
        for row in top..bottom {
            for channel in &raster.pixels[(row * width + left) * 4..(row * width + right) * 4] {
                pixels.push(((u16::from(*channel) * u16::from(rgba[3]) + 127) / 255) as u8);
            }
        }
        let mark = Arc::new(
            ImageData::from_rgba((right - left) as u32, (bottom - top) as u32, pixels.into())
                .unwrap(),
        );
        if self.marks.len() >= 32 {
            self.marks.clear();
        }
        self.marks.insert((font, rgba), mark.clone());
        Some(mark)
    }
}

macro_rules! boolean_control {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(Widget);
        impl $name {
            pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
                Self(Widget::new(
                    key,
                    Kind::$kind {
                        text: label.into(),
                        initial: false,
                        disabled: false,
                    },
                ))
            }
            /// Seeds the retained value once per widget key.
            pub fn checked(mut self, value: bool) -> Self {
                if let Kind::$kind { initial, .. } = &mut self.0.kind {
                    *initial = value;
                }
                self
            }
            pub fn disabled(mut self, value: bool) -> Self {
                if let Kind::$kind { disabled, .. } = &mut self.0.kind {
                    *disabled = value;
                }
                self
            }
            control_style_methods!();
            layout_methods!();
        }
        impl From<$name> for Widget {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
boolean_control!(Checkbox, Checkbox);
boolean_control!(Switch, Switch);

#[derive(Clone, Debug)]
pub struct RadioGroup(Widget);
impl RadioGroup {
    pub fn new(key: impl Into<String>) -> Self {
        Self(Widget::new(
            key,
            Kind::RadioGroup {
                initial_selection: None,
                disabled: false,
            },
        ))
    }
    pub fn child(mut self, option: RadioOption) -> Self {
        self.0.children.push(option.0);
        self
    }
    pub fn gap(mut self, value: f32) -> Self {
        self.0.layout.gap = Some(value.max(0.0));
        self
    }
    /// Seeds selection once; option keys are also the values reported in change events.
    pub fn selected(mut self, key: impl Into<String>) -> Self {
        if let Kind::RadioGroup {
            initial_selection, ..
        } = &mut self.0.kind
        {
            *initial_selection = Some(key.into());
        }
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        if let Kind::RadioGroup { disabled, .. } = &mut self.0.kind {
            *disabled = value;
        }
        self
    }
    pub fn option_style(mut self, style: StyleSheet) -> Self {
        self.0.item_style = Some(Arc::new(style));
        self
    }
    control_style_methods!();
    layout_methods!();
}
impl From<RadioGroup> for Widget {
    fn from(value: RadioGroup) -> Self {
        value.0
    }
}

#[derive(Clone, Debug)]
pub struct RadioOption(Widget);
impl RadioOption {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self(Widget::new(
            key,
            Kind::RadioOption {
                text: label.into(),
                disabled: false,
            },
        ))
    }
    pub fn disabled(mut self, value: bool) -> Self {
        if let Kind::RadioOption { disabled, .. } = &mut self.0.kind {
            *disabled = value;
        }
        self
    }
    control_style_methods!();
    layout_methods!();
}
impl From<RadioOption> for Widget {
    fn from(value: RadioOption) -> Self {
        value.0
    }
}

/// State-independent geometry; colors and surfaces use component style rules.
#[derive(Clone, Debug, PartialEq)]
pub struct FormMetrics {
    pub indicator_size: f32,
    pub switch_width: f32,
    pub indicator_gap: f32,
    pub padding: Insets,
    pub group_gap: f32,
}
impl Default for FormMetrics {
    fn default() -> Self {
        Self {
            indicator_size: 18.0,
            switch_width: 34.0,
            indicator_gap: 8.0,
            padding: Insets::xy(8.0, 6.0),
            group_gap: 4.0,
        }
    }
}
impl FormMetrics {
    pub(super) fn validate(&self) -> Result<(), LayoutError> {
        if [
            self.indicator_size,
            self.switch_width,
            self.indicator_gap,
            self.group_gap,
            self.padding.left,
            self.padding.right,
            self.padding.top,
            self.padding.bottom,
        ]
        .into_iter()
        .all(|v| v.is_finite() && v >= 0.0)
            && self.switch_width >= self.indicator_size
        {
            Ok(())
        } else {
            Err(LayoutError::InvalidStyle("form metrics".into()))
        }
    }
}

pub(super) fn validate(widget: &Widget, radio_parent: bool) -> Result<(), LayoutError> {
    if radio_parent != matches!(widget.kind, Kind::RadioOption { .. }) {
        return Err(LayoutError::InvalidRadioOption(widget.key.clone()));
    }
    for child in &widget.children {
        validate(child, matches!(widget.kind, Kind::RadioGroup { .. }))?;
    }
    Ok(())
}
pub(super) fn is_choice(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Checkbox { .. } | Kind::Switch { .. } | Kind::RadioOption { .. }
    )
}
pub(super) fn indicator(kind: &Kind, theme: &Theme) -> Size {
    Size::new(
        if matches!(kind, Kind::Switch { .. }) {
            theme.forms.switch_width
        } else {
            theme.forms.indicator_size
        },
        theme.forms.indicator_size,
    )
}
pub(super) fn radio_entry<'a>(
    group: &str,
    controls: &'a [ControlRegion],
    state: &crate::input::State,
) -> Option<&'a ControlRegion> {
    let eligible = |c: &&ControlRegion| {
        !c.disabled
            && matches!(&c.kind, ControlKind::RadioOption { group: owner } if owner == group)
    };
    controls
        .iter()
        .filter(eligible)
        .find(|c| state.radios.get(group).and_then(|v| v.as_deref()) == Some(c.key.as_str()))
        .or_else(|| controls.iter().find(eligible))
}
pub(super) fn tab_stops<'a>(
    controls: &'a [ControlRegion],
    state: &crate::input::State,
) -> Vec<&'a ControlRegion> {
    let mut entries = HashMap::new();
    for control in controls.iter().filter(|c| !c.disabled) {
        if let ControlKind::RadioOption { group } = &control.kind {
            if state.radios.get(group).and_then(|value| value.as_deref())
                == Some(control.key.as_str())
            {
                entries.insert(group, &control.key);
            } else {
                entries.entry(group).or_insert(&control.key);
            }
        }
    }
    controls
        .iter()
        .filter(|control| {
            !control.disabled
                && match &control.kind {
                    ControlKind::ListItem { .. } | ControlKind::RadioGroup => false,
                    ControlKind::RadioOption { group } => {
                        entries.get(group).is_some_and(|key| **key == control.key)
                    }
                    _ => true,
                }
        })
        .collect()
}

impl UiView {
    pub(super) fn sync_forms(&mut self) {
        fn visit(
            widget: &Widget,
            state: &mut crate::input::State,
            checks: &mut HashSet<String>,
            groups: &mut HashSet<String>,
            switches: &mut HashSet<String>,
        ) {
            match &widget.kind {
                Kind::Checkbox { initial, .. } | Kind::Switch { initial, .. } => {
                    checks.insert(widget.key.clone());
                    state.checked.entry(widget.key.clone()).or_insert(*initial);
                    if matches!(widget.kind, Kind::Switch { .. }) {
                        switches.insert(widget.key.clone());
                        let checked = state.checked[&widget.key];
                        state
                            .switches
                            .entry(widget.key.clone())
                            .or_insert_with(|| crate::input::SwitchMotion::new(checked));
                    }
                }
                Kind::RadioGroup {
                    initial_selection, ..
                } => {
                    groups.insert(widget.key.clone());
                    let value = state
                        .radios
                        .entry(widget.key.clone())
                        .or_insert_with(|| initial_selection.clone());
                    if value.as_ref().is_some_and(|key| {
                        !widget
                            .children
                            .iter()
                            .any(|c| &c.key == key && matches!(c.kind, Kind::RadioOption { .. }))
                    }) {
                        *value = None;
                    }
                }
                _ => {}
            }
            for child in &widget.children {
                visit(child, state, checks, groups, switches);
            }
        }
        let mut checks = HashSet::new();
        let mut groups = HashSet::new();
        let mut switches = HashSet::new();
        visit(
            &self.root,
            &mut self.state,
            &mut checks,
            &mut groups,
            &mut switches,
        );
        self.state.checked.retain(|key, _| checks.contains(key));
        self.state.radios.retain(|key, _| groups.contains(key));
        self.state.switches.retain(|key, _| switches.contains(key));
    }
    /// Opt into host-clock-driven switch motion. Zero snaps immediately (the default).
    pub fn set_switch_animation_duration(&mut self, duration: Duration) {
        if self.state.switch_duration == duration {
            return;
        }
        self.state.switch_duration = duration;
        if duration.is_zero() {
            let active = self.has_animations();
            for motion in self.state.switches.values_mut() {
                motion.snap();
            }
            if active {
                self.invalidate();
            }
        } else {
            for motion in self
                .state
                .switches
                .values_mut()
                .filter(|motion| motion.active())
            {
                motion.retime();
            }
        }
    }
    pub fn has_animations(&self) -> bool {
        self.state
            .switches
            .values()
            .any(crate::input::SwitchMotion::active)
    }
    /// Advance visual motion without changing values, input geometry, or emitting events.
    pub fn advance_animations(&mut self, delta: Duration) -> bool {
        self.sync_state();
        let mut changed = false;
        for motion in self.state.switches.values_mut() {
            changed |= motion.advance(delta, self.state.switch_duration);
        }
        if changed {
            self.invalidate();
        }
        changed
    }
    fn update_checked(&mut self, key: &str, checked: bool) {
        self.state.checked.insert(key.into(), checked);
        if let Some(motion) = self.state.switches.get_mut(key) {
            motion.retarget(checked, self.state.switch_duration);
        }
    }
    pub fn checked(&self, key: &str) -> Option<bool> {
        self.state.checked.get(key).copied()
    }
    /// Update retained state without emitting a user change event; disabled controls may be updated.
    pub fn set_checked(&mut self, key: &str, value: bool) -> Result<bool, InteractionError> {
        self.sync_state();
        let scene = self
            .input_scene
            .clone()
            .ok_or(InteractionError::NotPrepared)?;
        if !scene
            .controls
            .iter()
            .any(|c| c.key == key && matches!(c.kind, ControlKind::Checkbox | ControlKind::Switch))
        {
            return Err(InteractionError::NotCheckable(key.into()));
        }
        let changed = self.checked(key) != Some(value);
        if changed {
            self.update_checked(key, value);
            self.cancel_form_press(key, scene);
            self.invalidate();
        }
        Ok(changed)
    }
    pub fn radio_selected(&self, group: &str) -> Option<&str> {
        self.state.radios.get(group).and_then(|v| v.as_deref())
    }
    /// Set or clear a radio value without a user event; disabled options remain valid values.
    pub fn set_radio_selected(
        &mut self,
        group: &str,
        option: Option<&str>,
    ) -> Result<bool, InteractionError> {
        self.sync_state();
        let scene = self
            .input_scene
            .clone()
            .ok_or(InteractionError::NotPrepared)?;
        if !scene
            .controls
            .iter()
            .any(|c| c.key == group && matches!(c.kind, ControlKind::RadioGroup))
        {
            return Err(InteractionError::NotRadioGroup(group.into()));
        }
        if let Some(option) = option && !scene.controls.iter().any(|c| {
            c.key == option
                && matches!(&c.kind, ControlKind::RadioOption { group: owner } if owner == group)
        }) {
            return Err(InteractionError::InvalidRadioOption {
                group: group.into(),
                option: option.into(),
            });
        }
        let changed = self.radio_selected(group) != option;
        if changed {
            self.state
                .radios
                .insert(group.into(), option.map(str::to_owned));
            self.cancel_form_press(group, scene);
            self.invalidate();
        }
        Ok(changed)
    }
    fn cancel_form_press(&mut self, key: &str, scene: interaction::InputScene) {
        if self.state.pressed.as_ref().is_some_and(|pressed| {
            pressed == key
                || scene.controls.iter().any(|c| {
                    &c.key == pressed
                        && matches!(&c.kind, ControlKind::RadioOption { group } if group == key)
                })
        }) {
            self.state.pressed = None;
            self.state.choice_space = None;
        }
    }
    pub(super) fn activate_choice(&mut self, control: &ControlRegion, out: &mut Vec<UiEvent>) {
        if control.disabled
            || !matches!(
                control.kind,
                ControlKind::Checkbox | ControlKind::Switch | ControlKind::RadioOption { .. }
            )
        {
            return;
        }
        self.state.pressed = None;
        self.state.choice_space = None;
        match &control.kind {
            ControlKind::Checkbox | ControlKind::Switch => {
                let checked = !self.checked(&control.key).unwrap_or(false);
                self.update_checked(&control.key, checked);
                out.push(UiEvent::CheckedChanged {
                    key: control.key.clone(),
                    checked,
                });
            }
            ControlKind::RadioOption { group }
                if self.radio_selected(group) != Some(control.key.as_str()) =>
            {
                self.state
                    .radios
                    .insert(group.clone(), Some(control.key.clone()));
                out.push(UiEvent::RadioChanged {
                    key: group.clone(),
                    selected: Some(control.key.clone()),
                });
            }
            _ => {}
        }
    }
    pub(super) fn navigate_radio(
        &mut self,
        group: &str,
        current: &str,
        key: Key,
        scene: &interaction::InputScene,
        out: &mut Vec<UiEvent>,
    ) {
        let options: Vec<_> = scene.controls.iter().filter(|c| !c.disabled && matches!(&c.kind, ControlKind::RadioOption { group: owner } if owner == group)).collect();
        if options.is_empty() {
            return;
        }
        let index = options.iter().position(|c| c.key == current).unwrap_or(0);
        let next = match key {
            Key::Home => 0,
            Key::End => options.len() - 1,
            Key::Up | Key::Left => (index + options.len() - 1) % options.len(),
            Key::Down | Key::Right => (index + 1) % options.len(),
            _ => return,
        };
        self.change_focus(Some(options[next].key.clone()), out);
        self.activate_choice(options[next], out);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_choice(
    widget: &Widget,
    owner: Option<&Widget>,
    rect: Rect,
    content: Rect,
    clip: Rect,
    namespace: &str,
    visual: &ResolvedStyle,
    state: &mut crate::input::State,
    theme: &Theme,
    ancestors: &[String],
    measure: &mut impl FnMut(&Font, &str, Color) -> Size,
    out: &mut PreparedView,
    assets: &mut Assets,
    glyph_text: &mut Option<TextSystem>,
) {
    let key = format!("{namespace}/{}", widget.key);
    let (text, kind) = match &widget.kind {
        Kind::Checkbox { text, .. } => (text, ControlKind::Checkbox),
        Kind::Switch { text, .. } => (text, ControlKind::Switch),
        Kind::RadioOption { text, .. } => (
            text,
            ControlKind::RadioOption {
                group: owner.unwrap().key.clone(),
            },
        ),
        _ => unreachable!(),
    };
    let size = indicator(&widget.kind, theme);
    let indicator_rect = Rect::new(
        content.origin.x,
        content.origin.y + (content.size.height - size.height).max(0.0) * 0.5,
        size.width,
        size.height,
    );
    let checked = match &kind {
        ControlKind::RadioOption { group } => {
            state.radios.get(group).and_then(|v| v.as_deref()) == Some(widget.key.as_str())
        }
        _ => state.checked.get(&widget.key).copied().unwrap_or(false),
    };
    let mut position = f32::from(checked);
    let mut surface = visual.surface;
    if matches!(kind, ControlKind::Switch)
        && let Some(motion) = state.switches.get_mut(&widget.key)
    {
        position = motion.position;
        surface.fill = motion.fill(surface.fill);
    }
    out.items.push(PaintItem::Card {
        key: format!("{key}/indicator"),
        rect: indicator_rect,
        clip,
        style: surface,
    });
    let inset = size.height * 0.22;
    let dot = (size.height - 2.0 * inset).max(0.0);
    if matches!(kind, ControlKind::Switch)
        || (checked && matches!(kind, ControlKind::RadioOption { .. }))
    {
        let x = inset + (size.width - inset * 2.0 - dot) * position;
        out.items.push(PaintItem::Card {
            key: format!("{key}/mark"),
            rect: Rect::new(
                indicator_rect.origin.x + x,
                indicator_rect.origin.y + inset,
                dot,
                dot,
            ),
            clip,
            style: CardStyle {
                fill: visual.text,
                border: visual.text,
                border_width: 0.0,
                radius: dot * 0.5,
            },
        });
    } else if checked
        && let Some(mark) = assets.checkmark(size.height, &visual.font, visual.text, glyph_text)
    {
        out.items.push(PaintItem::Image {
            key: format!("{key}/mark"),
            rect: Rect::new(
                (indicator_rect.origin.x + (size.width - mark.width() as f32) * 0.5).round(),
                (indicator_rect.origin.y + (size.height - mark.height() as f32) * 0.5).round(),
                mark.width() as f32,
                mark.height() as f32,
            ),
            clip: clip.intersection(indicator_rect),
            data: mark,
        });
    }
    let left = size.width + theme.forms.indicator_gap;
    let label_rect = Rect::new(
        content.origin.x + left,
        content.origin.y,
        (content.size.width - left).max(0.0),
        content.size.height,
    );
    let (label, measured) = crate::text::fit_with_measure(
        text,
        label_rect.size.width,
        TextOverflow::EllipsisEnd,
        |value| measure(&visual.font, value, visual.text),
    );
    out.items.push(PaintItem::Text {
        key: format!("{key}/label"),
        rect: Rect::new(
            label_rect.origin.x,
            label_rect.origin.y + (label_rect.size.height - measured.height).max(0.0) * 0.5,
            measured.width,
            measured.height,
        ),
        clip: clip.intersection(label_rect),
        text: label,
        font: visual.font.clone(),
        color: visual.text,
    });
    out.controls.push(ControlRegion {
        key: widget.key.clone(),
        rect,
        clip,
        content,
        kind,
        disabled: visual.state.disabled,
        scroll_ancestors: ancestors.to_vec(),
        line: None,
        text_origin: label_rect.origin,
        caret: Rect::default(),
    });
}
