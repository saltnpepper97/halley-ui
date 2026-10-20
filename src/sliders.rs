#![doc = include_str!("../docs/components/slider.md")]
use super::*;
use crate::{InputEvent, Key, UiEvent};

/// Horizontal numeric control. `value` seeds retained state once per key.
#[derive(Clone, Debug)]
pub struct Slider(Widget);
impl Slider {
    pub fn new(key: impl Into<String>, min: f64, max: f64) -> Self {
        Self(Widget::new(
            key,
            Kind::Slider {
                label: String::new(),
                initial: min,
                range: SliderRange {
                    min,
                    max,
                    step: None,
                },
                disabled: false,
            },
        ))
    }
    pub fn label(mut self, value: impl Into<String>) -> Self {
        if let Kind::Slider { label, .. } = &mut self.0.kind {
            *label = value.into();
        }
        self
    }
    pub fn value(mut self, value: f64) -> Self {
        if let Kind::Slider { initial, .. } = &mut self.0.kind {
            *initial = value;
        }
        self
    }
    /// Optional positive step, anchored at the minimum. Both endpoints are reachable.
    pub fn step(mut self, value: f64) -> Self {
        if let Kind::Slider { range, .. } = &mut self.0.kind {
            range.step = Some(value);
        }
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        if let Kind::Slider { disabled, .. } = &mut self.0.kind {
            *disabled = value;
        }
        self
    }
    control_style_methods!();
    layout_methods!();
}
impl From<Slider> for Widget {
    fn from(value: Slider) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderRange {
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
}
impl SliderRange {
    fn valid(self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && self.min < self.max
            && (self.max - self.min).is_finite()
            && self.step.is_none_or(|s| s.is_finite() && s > 0.0)
    }
    fn normalize(self, value: f64) -> f64 {
        let value = value.clamp(self.min, self.max);
        if value == self.min || value == self.max {
            return value;
        }
        let Some(step) = self.step else {
            return value;
        };
        let ticks = ((value - self.min) / step).round();
        // A grid finer than floating-point precision is effectively continuous.
        if !ticks.is_finite() {
            return value;
        }
        let snapped = (self.min + ticks * step).clamp(self.min, self.max);
        if self.max - value < (snapped - value).abs() {
            self.max
        } else {
            snapped
        }
    }
    pub(crate) fn increment(self) -> f64 {
        self.step.unwrap_or_else(|| {
            let span = self.max - self.min;
            let increment = span / 100.0;
            if increment > 0.0 { increment } else { span }
        })
    }
    pub(crate) fn jump(self) -> f64 {
        self.increment().max((self.max - self.min) / 10.0)
    }
    fn adjusted(self, value: f64, key: Key) -> f64 {
        let step = self.increment();
        let next = match key {
            Key::Home => self.min,
            Key::End => self.max,
            Key::Left | Key::Down if value == self.max && self.step.is_some() => {
                let quotient = (self.max - self.min) / step;
                let nearest = quotient.round();
                let ticks =
                    if (quotient - nearest).abs() <= f64::EPSILON * 4.0 * quotient.abs().max(1.0) {
                        nearest - 1.0
                    } else {
                        quotient.floor()
                    };
                if ticks.is_finite() {
                    self.min + ticks * step
                } else {
                    value - step
                }
            }
            Key::Left | Key::Down => value - step,
            Key::Right | Key::Up => value + step,
            Key::PageDown => value - self.jump(),
            Key::PageUp => value + self.jump(),
            _ => value,
        };
        self.normalize(next)
    }
}

/// Numeric semantics shared by prepared views and accessibility adapters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericValue {
    pub value: f64,
    pub range: SliderRange,
}
#[derive(Clone, Copy)]
pub(crate) struct SliderState {
    pub range: SliderRange,
    pub value: f64,
}
pub(crate) struct SliderDrag {
    pub key: String,
    pub grab_offset: f32,
    pub last_x: f32,
}

pub(super) fn validate(widget: &Widget) -> Result<(), LayoutError> {
    if let Kind::Slider { range, initial, .. } = widget.kind
        && (!range.valid() || !initial.is_finite())
    {
        return Err(LayoutError::InvalidSlider(widget.key.clone()));
    }
    for child in &widget.children {
        validate(child)?;
    }
    Ok(())
}
impl UiView {
    pub(super) fn sync_sliders(&mut self) {
        fn visit(widget: &Widget, state: &mut crate::input::State, live: &mut HashSet<String>) {
            if let Kind::Slider {
                range,
                initial,
                disabled,
                ..
            } = widget.kind
                && range.valid()
                && initial.is_finite()
            {
                live.insert(widget.key.clone());
                let retained = state
                    .sliders
                    .entry(widget.key.clone())
                    .or_insert(SliderState {
                        range,
                        value: range.normalize(initial),
                    });
                if (disabled || retained.range != range)
                    && state
                        .slider_drag
                        .as_ref()
                        .is_some_and(|d| d.key == widget.key)
                {
                    state.slider_drag = None;
                    state.pressed = None;
                }
                retained.range = range;
                retained.value = range.normalize(retained.value);
            }
            for child in &widget.children {
                visit(child, state, live);
            }
        }
        let mut live = HashSet::new();
        visit(&self.root, &mut self.state, &mut live);
        self.state.sliders.retain(|key, _| live.contains(key));
        if self
            .state
            .slider_drag
            .as_ref()
            .is_some_and(|d| !live.contains(&d.key))
        {
            self.state.slider_drag = None;
            self.state.pressed = None;
        }
    }
    pub fn slider_value(&self, key: &str) -> Option<f64> {
        self.state.sliders.get(key).map(|s| s.value)
    }
    /// Silent programmatic update. Finite values are clamped and snapped to the range.
    pub fn set_slider_value(&mut self, key: &str, value: f64) -> Result<bool, InteractionError> {
        self.sync_state();
        if !value.is_finite() {
            return Err(InteractionError::InvalidNumericValue);
        }
        let slider = self
            .state
            .sliders
            .get_mut(key)
            .ok_or_else(|| InteractionError::NotSlider(key.into()))?;
        let value = slider.range.normalize(value);
        let changed = slider.value != value;
        slider.value = value;
        if changed {
            if self
                .state
                .slider_drag
                .as_ref()
                .is_some_and(|d| d.key == key)
            {
                self.state.slider_drag = None;
                self.state.pressed = None;
            }
            self.invalidate();
        }
        Ok(changed)
    }
    pub(super) fn user_slider_value(&mut self, key: &str, value: f64, out: &mut Vec<UiEvent>) {
        if !value.is_finite() {
            return;
        }
        if let Some(slider) = self.state.sliders.get_mut(key) {
            let value = slider.range.normalize(value);
            if slider.value != value {
                slider.value = value;
                out.push(UiEvent::SliderChanged {
                    key: key.into(),
                    value,
                });
            }
        }
    }
    pub(super) fn slider_key(&mut self, key: &str, command: Key, out: &mut Vec<UiEvent>) {
        if !matches!(
            command,
            Key::Left
                | Key::Right
                | Key::Up
                | Key::Down
                | Key::Home
                | Key::End
                | Key::PageUp
                | Key::PageDown
        ) {
            return;
        }
        if self
            .state
            .slider_drag
            .as_ref()
            .is_some_and(|d| d.key == key)
        {
            self.state.slider_drag = None;
            self.state.pressed = None;
        }
        if let Some(slider) = self.state.sliders.get(key) {
            self.user_slider_value(key, slider.range.adjusted(slider.value, command), out);
        }
    }
    pub(super) fn slider_pointer(
        &mut self,
        control: &ControlRegion,
        event: &InputEvent,
        out: &mut Vec<UiEvent>,
    ) {
        let ControlKind::Slider { range, thumb_size } = control.kind else {
            return;
        };
        let band = control.content;
        let travel = (band.size.width - thumb_size).max(0.0);
        if travel <= 0.0 {
            return;
        }
        let origin = band.origin.x + thumb_size * 0.5;
        let value = self.slider_value(&control.key).unwrap_or(range.min);
        let center = origin + ((value - range.min) / (range.max - range.min)) as f32 * travel;
        let position = match event {
            InputEvent::PointerDown { position, .. } if band.contains(*position) => {
                let on_thumb = (position.x - center).abs() <= thumb_size * 0.5;
                let grab_offset = if on_thumb { position.x - center } else { 0.0 };
                self.state.slider_drag = Some(SliderDrag {
                    key: control.key.clone(),
                    grab_offset,
                    last_x: position.x,
                });
                if on_thumb {
                    return;
                }
                *position
            }
            InputEvent::PointerMove { position } | InputEvent::PointerUp { position } => *position,
            _ => return,
        };
        if !position.x.is_finite() || !position.y.is_finite() {
            return;
        }
        let Some(drag) = self
            .state
            .slider_drag
            .as_mut()
            .filter(|d| d.key == control.key)
        else {
            return;
        };
        if !matches!(event, InputEvent::PointerDown { .. }) && position.x == drag.last_x {
            return;
        }
        drag.last_x = position.x;
        let fraction = ((position.x - drag.grab_offset - origin) / travel).clamp(0.0, 1.0) as f64;
        self.user_slider_value(
            &control.key,
            range.min + fraction * (range.max - range.min),
            out,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint(
    widget: &Widget,
    rect: Rect,
    content: Rect,
    clip: Rect,
    namespace: &str,
    visual: &ResolvedStyle,
    state: &crate::input::State,
    theme: &Theme,
    ancestors: &[String],
    measure: &mut impl FnMut(&Font, &str, Color) -> Size,
    out: &mut PreparedView,
) {
    let Kind::Slider { label, range, .. } = &widget.kind else {
        return;
    };
    let key = format!("{namespace}/{}", widget.key);
    let thumb = theme
        .forms
        .slider_thumb_size
        .min(content.size.width)
        .min(content.size.height);
    let band = Rect::new(
        content.origin.x,
        content.origin.y + (content.size.height - thumb).max(0.0),
        content.size.width,
        thumb,
    );
    let track_height = theme.forms.slider_track_height.min(thumb);
    let travel = (content.size.width - thumb).max(0.0);
    let value = state.sliders[&widget.key].value;
    let fraction = ((value - range.min) / (range.max - range.min)) as f32;
    let x = band.origin.x + thumb * 0.5;
    let y = band.origin.y + (thumb - track_height) * 0.5;
    let mut track_style = visual.surface;
    track_style.radius = track_height * 0.5;
    let mut filled = track_style;
    filled.fill = visual.selection;
    filled.border_width = 0.0;
    let mut thumb_style = visual.surface;
    thumb_style.fill = visual.text;
    thumb_style.radius = thumb * 0.5;
    out.items.push(PaintItem::Card {
        key: format!("{key}/track"),
        rect: Rect::new(x, y, travel, track_height),
        clip,
        style: track_style,
    });
    out.items.push(PaintItem::Card {
        key: format!("{key}/fill"),
        rect: Rect::new(x, y, travel * fraction, track_height),
        clip,
        style: filled,
    });
    out.items.push(PaintItem::Card {
        key: format!("{key}/thumb"),
        rect: Rect::new(
            band.origin.x + travel * fraction,
            band.origin.y,
            thumb,
            thumb,
        ),
        clip,
        style: thumb_style,
    });
    if !label.is_empty() {
        let label_rect = Rect::new(
            content.origin.x,
            content.origin.y,
            content.size.width,
            (band.origin.y - content.origin.y - theme.forms.indicator_gap).max(0.0),
        );
        let (text, size) = crate::text::fit_with_measure(
            label,
            label_rect.size.width,
            TextOverflow::EllipsisEnd,
            |s| measure(&visual.font, s, visual.text),
        );
        out.items.push(PaintItem::Text {
            key: format!("{key}/label"),
            rect: Rect::new(
                label_rect.origin.x,
                label_rect.origin.y,
                size.width,
                size.height,
            ),
            clip: clip.intersection(label_rect),
            text,
            font: visual.font.clone(),
            color: visual.text,
        });
    }
    out.controls.push(ControlRegion {
        key: widget.key.clone(),
        rect,
        clip,
        content: band,
        kind: ControlKind::Slider {
            range: *range,
            thumb_size: thumb,
        },
        disabled: visual.state.disabled,
        scroll_ancestors: ancestors.to_vec(),
        line: None,
        text_origin: content.origin,
        caret: Rect::default(),
    });
}
