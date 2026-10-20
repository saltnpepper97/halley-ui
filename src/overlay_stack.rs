use super::*;

struct OpenLayer {
    definition: Overlay,
    view: UiView,
    return_focus: Option<String>,
    initial: bool,
    placement: Placement,
}

/// Composes retained views into one paint snapshot. Only the front layer owns input.
pub struct UiStack {
    base: UiView,
    layers: Vec<OpenLayer>,
    snapshots: Vec<(LayerId, PreparedView)>,
    cached: Option<PreparedView>,
    theme: Option<Theme>,
    bounds: Option<Rect>,
    revision: u64,
    dirty: bool,
    input_valid: bool,
    geometry_valid: bool,
    window_focused: bool,
    suppress_release: bool,
    menu_space: Option<(String, Option<String>)>,
    paste_serial: u64,
    pending_paste: Option<(u64, LayerId, u64)>,
    restore: Option<(LayerId, Option<String>)>,
    events: Vec<LayerEvent>,
}
impl UiStack {
    pub fn new(base: UiView) -> Self {
        let window_focused = base.state.window_focused;
        Self {
            base,
            layers: Vec::new(),
            snapshots: Vec::new(),
            cached: None,
            theme: None,
            bounds: None,
            revision: 0,
            dirty: true,
            input_valid: false,
            geometry_valid: false,
            window_focused,
            suppress_release: false,
            menu_space: None,
            paste_serial: 0,
            pending_paste: None,
            restore: None,
            events: Vec::new(),
        }
    }
    pub fn base(&self) -> &UiView {
        &self.base
    }
    /// Prepare again after mutating base content before opening overlays or routing input.
    pub fn base_mut(&mut self) -> &mut UiView {
        self.invalidate(true);
        self.geometry_valid = false;
        &mut self.base
    }
    pub fn view(&self, layer: &LayerId) -> Option<&UiView> {
        match layer {
            LayerId::Base => Some(&self.base),
            LayerId::Overlay(key) => self
                .layers
                .iter()
                .find(|l| &l.definition.key == key)
                .map(|l| &l.view),
        }
    }
    fn view_mut(&mut self, layer: &LayerId) -> Option<&mut UiView> {
        match layer {
            LayerId::Base => Some(&mut self.base),
            LayerId::Overlay(key) => self
                .layers
                .iter_mut()
                .find(|l| &l.definition.key == key)
                .map(|l| &mut l.view),
        }
    }
    pub fn active_layer(&self) -> LayerId {
        self.layers
            .last()
            .map_or(LayerId::Base, |l| LayerId::overlay(&l.definition.key))
    }
    pub fn is_open(&self, key: &str) -> bool {
        self.layers.iter().any(|l| l.definition.key == key)
    }
    pub fn snapshot(&self, layer: &LayerId) -> Option<&PreparedView> {
        self.snapshots
            .iter()
            .find(|(id, _)| id == layer)
            .map(|(_, p)| p)
    }
    pub fn placement(&self, key: &str) -> Option<Placement> {
        self.layers
            .iter()
            .find(|l| l.definition.key == key)
            .map(|l| l.placement)
    }
    pub fn needs_prepare(&self) -> bool {
        self.dirty
            || self.base.needs_prepare()
            || self.layers.iter().any(|l| l.view.needs_prepare())
    }
    /// Focus and anchor-loss events produced while preparing; drain after preparation.
    pub fn take_events(&mut self) -> Vec<LayerEvent> {
        std::mem::take(&mut self.events)
    }
    fn invalidate(&mut self, structural: bool) {
        self.dirty = true;
        if structural {
            self.input_valid = false;
            self.revision = self.revision.wrapping_add(1);
            self.menu_space = None;
            self.pending_paste = None;
            cancel_capture(&mut self.base);
            for layer in &mut self.layers {
                cancel_capture(&mut layer.view);
            }
        }
    }
    fn anchor_rect(&self, anchor: &OverlayAnchor, before: usize) -> Result<Rect, OverlayError> {
        match anchor {
            OverlayAnchor::Rect(rect) => Ok(*rect),
            OverlayAnchor::Widget { layer, key } => {
                let allowed = match layer {
                    LayerId::Base => true,
                    LayerId::Overlay(owner) => self
                        .layers
                        .iter()
                        .position(|l| &l.definition.key == owner)
                        .is_some_and(|i| i < before),
                };
                if allowed
                    && let Some(snapshot) = self.snapshot(layer)
                    && let Some(rect) = snapshot.rects.get(key)
                {
                    let clip = snapshot
                        .semantics
                        .iter()
                        .find(|s| &s.key == key)
                        .map_or(snapshot.bounds, |s| s.clip);
                    let visible = rect.intersection(clip);
                    if visible.size.width > 0.0 && visible.size.height > 0.0 {
                        return Ok(visible);
                    }
                }
                Err(OverlayError::UnknownAnchor {
                    layer: layer.clone(),
                    key: key.clone(),
                })
            }
        }
    }
    pub fn open(&mut self, overlay: impl Into<Overlay>) -> Result<Vec<LayerEvent>, OverlayError> {
        if !self.geometry_valid {
            return Err(OverlayError::NotPrepared);
        }
        let definition = overlay.into();
        definition.validate()?;
        if self.is_open(&definition.key) {
            return Err(OverlayError::DuplicateKey(definition.key));
        }
        if let Some(anchor) = &definition.anchor {
            self.anchor_rect(anchor, self.layers.len())?;
        }
        let active = self.active_layer();
        let return_focus = self
            .view(&active)
            .and_then(UiView::focused)
            .map(str::to_owned);
        if let Some(view) = self.view_mut(&active) {
            window_focus(view, false);
        }
        let key = definition.key.clone();
        let mut view =
            UiView::new(format!("{}/overlay/{key}", self.base.key)).anchor(Anchor::TopLeft);
        window_focus(&mut view, self.window_focused);
        self.layers.push(OpenLayer {
            definition,
            view,
            return_focus,
            initial: true,
            placement: Placement::Center,
        });
        self.menu_space = None;
        self.invalidate(true);
        Ok(vec![LayerEvent::Opened { key }])
    }
    /// Replace a definition without discarding state associated with its surviving widget keys.
    pub fn update(&mut self, overlay: impl Into<Overlay>) -> Result<(), OverlayError> {
        let definition = overlay.into();
        definition.validate()?;
        let i = self
            .layers
            .iter()
            .position(|l| l.definition.key == definition.key)
            .ok_or_else(|| OverlayError::UnknownOverlay(definition.key.clone()))?;
        if let Some(anchor) = &definition.anchor {
            self.anchor_rect(anchor, i)?;
        }
        self.layers[i].initial |= self.layers[i].definition.kind != definition.kind;
        self.layers[i].definition = definition;
        self.invalidate(true);
        self.geometry_valid = false;
        Ok(())
    }
    pub fn close(&mut self, key: &str) -> Result<Vec<LayerEvent>, OverlayError> {
        let index = self
            .layers
            .iter()
            .position(|l| l.definition.key == key)
            .ok_or_else(|| OverlayError::UnknownOverlay(key.into()))?;
        Ok(self.close_from(index, CloseReason::Programmatic))
    }
    pub fn close_top(&mut self) -> Vec<LayerEvent> {
        if self.layers.is_empty() {
            Vec::new()
        } else {
            self.close_from(self.layers.len() - 1, CloseReason::Programmatic)
        }
    }
    fn close_from(&mut self, index: usize, reason: CloseReason) -> Vec<LayerEvent> {
        let return_focus = self.layers[index].return_focus.clone();
        if let Some(layer) = self.layers.last_mut() {
            window_focus(&mut layer.view, false);
        }
        let mut events = Vec::new();
        while self.layers.len() > index {
            let layer = self.layers.pop().unwrap();
            events.push(LayerEvent::Closed {
                key: layer.definition.key,
                reason,
            });
        }
        let active = self.active_layer();
        let focused = self.window_focused;
        if let Some(view) = self.view_mut(&active) {
            window_focus(view, focused);
        }
        self.restore = Some((active, return_focus));
        self.snapshots.retain(|(id, _)| match id {
            LayerId::Base => true,
            LayerId::Overlay(key) => self.layers.iter().any(|l| &l.definition.key == key),
        });
        self.menu_space = None;
        self.invalidate(true);
        events
    }
    pub fn focus(&mut self, layer: &LayerId, key: &str) -> Result<Vec<LayerEvent>, OverlayError> {
        if layer != &self.active_layer() {
            return Err(OverlayError::InactiveLayer(layer.clone()));
        }
        let events = self.view_mut(layer).unwrap().focus(key)?;
        self.restore = None;
        self.invalidate(false);
        Ok(wrap(layer, events))
    }
    /// Programmatic updates may target an inactive layer; asynchronous input replies must use `handle_event_for`.
    pub fn set_text_value(
        &mut self,
        layer: &LayerId,
        key: &str,
        value: &str,
    ) -> Result<bool, OverlayError> {
        let view = self
            .view_mut(layer)
            .ok_or_else(|| OverlayError::InactiveLayer(layer.clone()))?;
        let changed = view.set_text_value(key, value);
        if changed {
            self.invalidate(false);
        }
        Ok(changed)
    }
    pub fn prepare(
        &mut self,
        bounds: Rect,
        theme: &Theme,
        text: &mut TextSystem,
    ) -> Result<&PreparedView, OverlayError> {
        self.prepare_with_color_measure(bounds, theme, |font, value, color| {
            text.raster(font, value, color.bytes())
                .map_or(Size::default(), |r| r.size())
        })
    }
    pub fn prepare_with_measure(
        &mut self,
        bounds: Rect,
        theme: &Theme,
        mut measure: impl FnMut(&Font, &str) -> Size,
    ) -> Result<&PreparedView, OverlayError> {
        self.prepare_with_color_measure(bounds, theme, |font, value, _| measure(font, value))
    }
    pub fn prepare_with_color_measure(
        &mut self,
        bounds: Rect,
        theme: &Theme,
        mut measure: impl FnMut(&Font, &str, Color) -> Size,
    ) -> Result<&PreparedView, OverlayError> {
        if !rect_valid(bounds) {
            self.input_valid = false;
            self.geometry_valid = false;
            self.dirty = true;
            return Err(OverlayError::InvalidGeometry);
        }
        if !self.needs_prepare()
            && self.bounds == Some(bounds)
            && self.theme.as_ref() == Some(theme)
        {
            return Ok(self.cached.as_ref().unwrap());
        }
        self.input_valid = false;
        self.geometry_valid = false;
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
        theme.validate()?;
        // Anchor loss can close a whole suffix. Restart to paint restored focus correctly.
        'restart: loop {
            self.snapshots.clear();
            let active = self.active_layer();
            window_focus(
                &mut self.base,
                self.window_focused && active == LayerId::Base,
            );
            let mut base = self
                .base
                .prepare_with_color_measure(bounds, theme, &mut measure)?
                .clone();
            if self
                .restore
                .as_ref()
                .is_some_and(|(layer, _)| *layer == LayerId::Base)
            {
                let (_, target) = self.restore.take().unwrap();
                self.events.extend(wrap(
                    &LayerId::Base,
                    restore_focus(&mut self.base, target.as_deref()),
                ));
                base = self
                    .base
                    .prepare_with_color_measure(bounds, theme, &mut measure)?
                    .clone();
            }
            self.snapshots.push((LayerId::Base, base));
            let mut combined = PreparedView {
                bounds,
                namespace: format!("{}/stack", self.base.key),
                content_revision: self.revision,
                ..Default::default()
            };
            append_paint(&mut combined, &self.snapshots[0].1, &LayerId::Base);
            for i in 0..self.layers.len() {
                let definition = self.layers[i].definition.clone();
                let anchor = if let Some(anchor) = &definition.anchor {
                    match self.anchor_rect(anchor, i) {
                        Ok(rect) => rect,
                        Err(_) => {
                            let events = self.close_from(i, CloseReason::AnchorLost);
                            self.events.extend(events);
                            continue 'restart;
                        }
                    }
                } else {
                    bounds
                };
                let layer_id = LayerId::overlay(&definition.key);
                let layer = &mut self.layers[i];
                window_focus(&mut layer.view, self.window_focused && layer_id == active);
                let mut layer_theme = theme.clone();
                if definition.kind == OverlayKind::Menu {
                    layer_theme.styles.list = theme.styles.menu.clone();
                    layer_theme.styles.list_item = theme.styles.menu_item.clone();
                }
                let margin = theme
                    .spacing
                    .medium
                    .min(bounds.size.width * 0.5)
                    .min(bounds.size.height * 0.5);
                let available = Rect::new(
                    bounds.origin.x + margin,
                    bounds.origin.y + margin,
                    (bounds.size.width - 2.0 * margin).max(0.0),
                    (bounds.size.height - 2.0 * margin).max(0.0),
                );
                layer
                    .view
                    .set_content(definition.widget(theme, available.size));
                let mut source = layer
                    .view
                    .prepare_with_color_measure(available, &layer_theme, &mut measure)?
                    .clone();
                let restore = if self.restore.as_ref().is_some_and(|(id, _)| id == &layer_id) {
                    self.restore.take().map(|(_, key)| key)
                } else {
                    None
                };
                if let Some(target) = restore {
                    self.events.extend(wrap(
                        &layer_id,
                        restore_focus(&mut layer.view, target.as_deref()),
                    ));
                    layer.initial = false;
                } else if layer.initial || (layer_id == active && layer.view.focused().is_none()) {
                    let target = definition.initial_focus.as_deref().or_else(|| {
                        if definition.kind == OverlayKind::Menu {
                            Some(definition.key.as_str())
                        } else {
                            None
                        }
                    });
                    let candidate = target
                        .filter(|key| source.controls.iter().any(|c| c.key == *key && !c.disabled))
                        .map(str::to_owned)
                        .or_else(|| {
                            source
                                .controls
                                .iter()
                                .find(|c| {
                                    !c.disabled && !matches!(c.kind, ControlKind::ListItem { .. })
                                })
                                .map(|c| c.key.clone())
                        });
                    self.events.extend(wrap(
                        &layer_id,
                        restore_focus(&mut layer.view, candidate.as_deref()),
                    ));
                    layer.initial = false;
                }
                source = layer
                    .view
                    .prepare_with_color_measure(available, &layer_theme, &mut measure)?
                    .clone();
                let (origin, placement) = place(
                    anchor,
                    source.bounds.size,
                    available,
                    definition.placement,
                    definition.offset,
                );
                layer.placement = placement;
                let offset = Point::new(
                    origin.x - source.bounds.origin.x,
                    origin.y - source.bounds.origin.y,
                );
                let mut placed = source.translated(offset);
                if definition.kind == OverlayKind::Dialog {
                    if let Some(root) = placed.semantics.first_mut() {
                        root.role = SemanticRole::Dialog;
                        root.modal = true;
                    }
                    combined.items.push(PaintItem::Card {
                        key: layer_id.qualify("backdrop"),
                        rect: bounds,
                        clip: bounds,
                        style: CardStyle {
                            fill: theme.overlays.backdrop,
                            border: Color::rgba(0.0, 0.0, 0.0, 0.0),
                            border_width: 0.0,
                            radius: 0.0,
                        },
                    });
                }
                if definition.kind == OverlayKind::Menu {
                    let options: HashSet<_> = placed.controls.iter().filter(|c| matches!(&c.kind, ControlKind::ListItem { list, .. } if list == &definition.key)).map(|c| c.key.clone()).collect();
                    for node in &mut placed.semantics {
                        if node.key == definition.key {
                            node.role = SemanticRole::Menu;
                            node.label = definition.title.clone();
                        } else if options.contains(&node.key) {
                            node.role = SemanticRole::MenuItem;
                        }
                    }
                }
                if layer_id == active && self.window_focused && placed.focused.is_none() {
                    placed.focused = placed.semantics.first().map(|s| s.key.clone());
                }
                append_paint(&mut combined, &placed, &layer_id);
                self.snapshots.push((layer_id, placed));
            }
            // Expose only the active interaction scope to accessibility and hit consumers.
            let (id, source) = self.snapshots.last().unwrap();
            append_input(&mut combined, source, id);
            self.cached = Some(combined);
            self.bounds = Some(bounds);
            self.theme = Some(theme.clone());
            self.dirty = false;
            self.input_valid = true;
            self.geometry_valid = true;
            return Ok(self.cached.as_ref().unwrap());
        }
    }
    pub fn handle_event(&mut self, event: InputEvent) -> Vec<LayerEvent> {
        self.route(event, Point::default())
    }
    /// Preserve the layer ID of an async clipboard request and use it for the reply.
    pub fn handle_event_for(&mut self, layer: &LayerId, event: InputEvent) -> Vec<LayerEvent> {
        if layer != &self.active_layer() {
            return Vec::new();
        }
        self.handle_event(event)
    }
    pub fn handle_event_in(
        &mut self,
        presented: &PreparedView,
        event: InputEvent,
    ) -> Vec<LayerEvent> {
        // Release/cancellation and native focus still matter after the frame was dismissed.
        if matches!(
            event,
            InputEvent::WindowFocus(_) | InputEvent::PointerCancel
        ) || (self.suppress_release && matches!(event, InputEvent::PointerUp { .. }))
        {
            return self.route(event, Point::default());
        }
        let Some(offset) = self.presentation_offset(presented) else {
            return Vec::new();
        };
        self.route(event, offset)
    }
    fn presentation_offset(&self, presented: &PreparedView) -> Option<Point> {
        let current = self.cached.as_ref()?;
        if !self.input_valid
            || current.namespace != presented.namespace
            || current.content_revision != presented.content_revision
            || current.bounds.size != presented.bounds.size
            || !rect_valid(presented.bounds)
        {
            return None;
        }
        Some(Point::new(
            presented.bounds.origin.x - current.bounds.origin.x,
            presented.bounds.origin.y - current.bounds.origin.y,
        ))
    }
    fn route(&mut self, mut event: InputEvent, offset: Point) -> Vec<LayerEvent> {
        if let InputEvent::WindowFocus(focused) = event {
            self.window_focused = focused;
            self.menu_space = None;
            if !focused {
                self.pending_paste = None;
            }
            let mut events = Vec::new();
            if !focused {
                while self
                    .layers
                    .last()
                    .is_some_and(|l| l.definition.dismiss_blur)
                {
                    events.extend(self.close_from(self.layers.len() - 1, CloseReason::WindowBlur));
                }
            }
            let active = self.active_layer();
            if let Some(view) = self.view_mut(&active) {
                window_focus(view, focused);
            }
            self.invalidate(false);
            return events;
        }
        if matches!(event, InputEvent::PointerCancel) {
            self.suppress_release = false;
            self.menu_space = None;
        }
        if matches!(event, InputEvent::PointerUp { .. }) && self.suppress_release {
            self.suppress_release = false;
            return Vec::new();
        }
        if !self.window_focused || !self.input_valid {
            return Vec::new();
        }
        let active = self.active_layer();
        if let InputEvent::Paste { request, .. } = &mut event {
            let Some((serial, owner, local)) = &self.pending_paste else {
                return Vec::new();
            };
            if request != serial || owner != &active {
                return Vec::new();
            }
            *request = *local;
            self.pending_paste = None;
        }
        let Some(snapshot) = self.snapshot(&active).cloned() else {
            return Vec::new();
        };
        let presented = snapshot.translated(offset);
        let mut events = Vec::new();
        if let Some(layer) = self.layers.last() {
            let definition = layer.definition.clone();
            if let InputEvent::PointerDown { position, .. } = event
                && !presented.bounds.contains(position)
            {
                self.suppress_release = true;
                if definition.dismiss_outside {
                    return self.close_from(self.layers.len() - 1, CloseReason::OutsidePress);
                }
                self.layers
                    .last_mut()
                    .unwrap()
                    .view
                    .handle_event(InputEvent::PointerCancel);
                self.invalidate(false);
                return Vec::new();
            }
            if let InputEvent::Wheel { position, .. } = event
                && !presented.bounds.contains(position)
            {
                return Vec::new();
            }
            if let InputEvent::KeyDown {
                key: Key::Escape, ..
            } = event
            {
                let composing = layer
                    .view
                    .focused()
                    .and_then(|key| layer.view.state.editors.get(key))
                    .is_some_and(|e| e.preedit.is_some());
                if !composing {
                    if definition.dismiss_escape {
                        return self.close_from(self.layers.len() - 1, CloseReason::Escape);
                    }
                    return Vec::new();
                }
            }
            if definition.kind == OverlayKind::Menu {
                if let InputEvent::PointerMove { position } = &event
                    && let Some(option) = presented.controls.iter().rev().find(|c| !c.disabled && c.rect.contains(*position)
                        && c.clip.contains(*position) && matches!(&c.kind, ControlKind::ListItem { list, .. } if list == &definition.key)) {
                    events.extend(self.layers.last_mut().unwrap().view.select(&definition.key, Some(&option.key)).unwrap_or_default());
                }
                if let InputEvent::KeyDown { key, modifiers } = &event
                    && !modifiers.control
                    && !modifiers.alt
                {
                    if *key == Key::Space {
                        if self.menu_space.is_none() {
                            self.menu_space = Some((
                                definition.key.clone(),
                                layer_selection(self.layers.last().unwrap()),
                            ));
                        }
                        return Vec::new();
                    }
                    if *key == Key::Enter {
                        let events = self
                            .layers
                            .last()
                            .unwrap()
                            .view
                            .activate_selected(&definition.key)
                            .unwrap_or_default();
                        return self.finish_events(&active, events);
                    }
                    if matches!(
                        key,
                        Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown
                    ) {
                        let enabled: Vec<_> = presented.controls.iter().filter(|c| !c.disabled && matches!(&c.kind, ControlKind::ListItem { list, .. } if list == &definition.key)).collect();
                        let view = &mut self.layers.last_mut().unwrap().view;
                        let selected = view.selected(&definition.key);
                        let wrapped = if *key == Key::Down
                            && enabled
                                .last()
                                .is_some_and(|c| Some(c.key.as_str()) == selected)
                        {
                            Some(Key::Home)
                        } else if *key == Key::Up
                            && enabled
                                .first()
                                .is_some_and(|c| Some(c.key.as_str()) == selected)
                        {
                            Some(Key::End)
                        } else {
                            None
                        };
                        let events = view
                            .navigate_list(&definition.key, wrapped.unwrap_or(*key))
                            .unwrap_or_default();
                        self.invalidate(false);
                        return wrap(&active, events);
                    }
                }
                if matches!(event, InputEvent::KeyUp { key: Key::Space }) {
                    if self.menu_space.take().is_some_and(|(key, item)| {
                        key == definition.key
                            && item == layer_selection(self.layers.last().unwrap())
                    }) {
                        let events = self
                            .layers
                            .last()
                            .unwrap()
                            .view
                            .activate_selected(&definition.key)
                            .unwrap_or_default();
                        return self.finish_events(&active, events);
                    }
                    return Vec::new();
                }
            }
        }
        events.extend(
            self.view_mut(&active)
                .unwrap()
                .handle_event_in(&presented, event),
        );
        if self.view(&active).unwrap().needs_prepare() {
            self.invalidate(false);
        }
        self.finish_events(&active, events)
    }
    fn finish_events(&mut self, active: &LayerId, events: Vec<UiEvent>) -> Vec<LayerEvent> {
        let mut out = Vec::new();
        if self
            .layers
            .last()
            .is_some_and(|l| l.definition.close_on_activate)
            && events.iter().any(|e| matches!(e, UiEvent::Activate(_)))
        {
            out.extend(self.close_from(self.layers.len() - 1, CloseReason::Activated));
        }
        for mut event in events {
            if let UiEvent::ClipboardRead { request } = &mut event {
                self.paste_serial = self.paste_serial.wrapping_add(1);
                self.pending_paste = Some((self.paste_serial, active.clone(), *request));
                *request = self.paste_serial;
            }
            out.push(LayerEvent::View {
                layer: active.clone(),
                event,
            });
        }
        out
    }
    pub fn ime_state(&self) -> Option<LayerEvent> {
        let current = self.cached.as_ref()?;
        self.ime_state_in(current)
    }
    pub fn ime_state_in(&self, presented: &PreparedView) -> Option<LayerEvent> {
        let offset = self.presentation_offset(presented)?;
        let active = self.active_layer();
        let snapshot = self.snapshot(&active)?.translated(offset);
        Some(LayerEvent::View {
            layer: active.clone(),
            event: self.view(&active)?.ime_state_in(&snapshot),
        })
    }
    #[cfg(feature = "accessibility")]
    pub fn handle_accessibility(
        &mut self,
        mut action: crate::accessibility::AccessibleAction,
    ) -> Vec<LayerEvent> {
        use crate::accessibility::AccessibleAction;
        if !self.input_valid || !self.window_focused {
            return Vec::new();
        }
        let active = self.active_layer();
        let prefix = active.prefix();
        let key = match &mut action {
            AccessibleAction::Focus(key)
            | AccessibleAction::Activate(key)
            | AccessibleAction::Reveal(key)
            | AccessibleAction::SetValue { key, .. }
            | AccessibleAction::SetSelection { key, .. }
            | AccessibleAction::Scroll { key, .. } => key,
        };
        let Some(local) = key.strip_prefix(&prefix) else {
            return Vec::new();
        };
        *key = local.to_owned();
        let events = self.view_mut(&active).unwrap().handle_accessibility(action);
        self.invalidate(false);
        self.finish_events(&active, events)
    }
}

fn layer_selection(layer: &OpenLayer) -> Option<String> {
    layer
        .view
        .selected(&layer.definition.key)
        .map(str::to_owned)
}
fn wrap(layer: &LayerId, events: Vec<UiEvent>) -> Vec<LayerEvent> {
    events
        .into_iter()
        .map(|event| LayerEvent::View {
            layer: layer.clone(),
            event,
        })
        .collect()
}
fn restore_focus(view: &mut UiView, key: Option<&str>) -> Vec<UiEvent> {
    if let Some(key) = key
        && let Ok(events) = view.focus(key)
    {
        return events;
    }
    view.clear_focus()
}
fn window_focus(view: &mut UiView, focused: bool) {
    if view.input_scene.is_some() {
        view.handle_event(InputEvent::WindowFocus(focused));
    } else {
        view.state.window_focused = focused;
        if !focused {
            view.state.pressed = None;
            view.state.dragging = false;
            view.state.pending_paste = None;
            for editor in view.state.editors.values_mut() {
                editor.preedit = None;
            }
        }
        view.invalidate();
    }
}
fn cancel_capture(view: &mut UiView) {
    if view.state.pressed.take().is_some() || view.state.dragging {
        view.state.dragging = false;
        view.invalidate();
    }
}
fn append_paint(out: &mut PreparedView, source: &PreparedView, layer: &LayerId) {
    for item in &source.items {
        let mut item = item.clone();
        match &mut item {
            PaintItem::Card { key, .. }
            | PaintItem::Text { key, .. }
            | PaintItem::Image { key, .. } => *key = layer.qualify(key),
        }
        out.items.push(item);
    }
    out.rects.extend(
        source
            .rects
            .iter()
            .map(|(key, rect)| (layer.qualify(key), *rect)),
    );
}
fn append_input(out: &mut PreparedView, source: &PreparedView, layer: &LayerId) {
    out.focused = source.focused.as_ref().map(|key| layer.qualify(key));
    out.hits = source
        .hits
        .iter()
        .cloned()
        .map(|mut hit| {
            hit.key = layer.qualify(&hit.key);
            hit
        })
        .collect();
    out.controls = source
        .controls
        .iter()
        .cloned()
        .map(|mut control| {
            control.key = layer.qualify(&control.key);
            control.scroll_ancestors = control
                .scroll_ancestors
                .iter()
                .map(|key| layer.qualify(key))
                .collect();
            if let ControlKind::ListItem { list, .. } = &mut control.kind {
                *list = layer.qualify(list);
            }
            control
        })
        .collect();
    out.scrolls = source
        .scrolls
        .iter()
        .cloned()
        .map(|mut scroll| {
            scroll.key = layer.qualify(&scroll.key);
            scroll.ancestors = scroll
                .ancestors
                .iter()
                .map(|key| layer.qualify(key))
                .collect();
            scroll
        })
        .collect();
    out.semantics = source
        .semantics
        .iter()
        .cloned()
        .map(|mut node| {
            node.key = layer.qualify(&node.key);
            node.children = node.children.iter().map(|key| layer.qualify(key)).collect();
            node.active_descendant = node
                .active_descendant
                .as_ref()
                .map(|key| layer.qualify(key));
            node
        })
        .collect();
}
