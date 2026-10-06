//! Retained input state; deliberately independent of a window system.
use super::*;
use crate::input::{Editor, InputEvent, Key, Modifiers, UiEvent};

#[derive(Clone, Default)]
pub(super) struct InputScene {
    controls: Vec<ControlRegion>,
    scrolls: Vec<ScrollRegion>,
}
impl From<&PreparedView> for InputScene {
    fn from(view: &PreparedView) -> Self {
        Self {
            controls: view.controls.clone(),
            scrolls: view.scrolls.clone(),
        }
    }
}
impl UiView {
    pub(super) fn sync_state(&mut self) {
        fn visit(
            widget: &Widget,
            state: &mut crate::input::State,
            inputs: &mut HashSet<String>,
            scrolls: &mut HashSet<String>,
            focusable: &mut HashSet<String>,
        ) {
            match &widget.kind {
                Kind::TextInput {
                    initial,
                    disabled,
                    read_only,
                    ..
                } => {
                    inputs.insert(widget.key.clone());
                    let editor = state
                        .editors
                        .entry(widget.key.clone())
                        .or_insert_with(|| Editor::new(initial));
                    if *disabled || *read_only {
                        editor.preedit = None;
                    }
                    if !disabled {
                        focusable.insert(widget.key.clone());
                    }
                }
                Kind::Button {
                    disabled: false, ..
                } => {
                    focusable.insert(widget.key.clone());
                }
                Kind::Scroll => {
                    scrolls.insert(widget.key.clone());
                }
                _ => {}
            }
            for child in &widget.children {
                visit(child, state, inputs, scrolls, focusable);
            }
        }
        let mut inputs = HashSet::new();
        let mut scrolls = HashSet::new();
        let mut focusable = HashSet::new();
        visit(
            &self.root,
            &mut self.state,
            &mut inputs,
            &mut scrolls,
            &mut focusable,
        );
        self.state.editors.retain(|key, _| inputs.contains(key));
        self.state.scroll.retain(|key, _| scrolls.contains(key));
        if self
            .state
            .focus
            .as_ref()
            .is_some_and(|key| !focusable.contains(key))
        {
            self.state.focus = None;
            self.state.pending_paste = None;
        }
        if self
            .state
            .pressed
            .as_ref()
            .is_some_and(|key| !focusable.contains(key))
        {
            self.state.pressed = None;
            self.state.dragging = false;
        }
    }
    pub fn focused(&self) -> Option<&str> {
        self.state.focus.as_deref()
    }
    pub fn text_value(&self, key: &str) -> Option<&str> {
        self.state
            .editors
            .get(key)
            .map(|editor| editor.value.as_str())
    }
    /// Programmatic replacement resets selection and cancels outstanding paste/composition.
    pub fn set_text_value(&mut self, key: &str, value: &str) -> bool {
        self.sync_state();
        let Some(editor) = self.state.editors.get_mut(key) else {
            return false;
        };
        let mut replacement = Editor::new(value);
        replacement.revision = editor.revision.wrapping_add(1);
        *editor = replacement;
        self.state.pending_paste = None;
        self.invalidate();
        true
    }
    pub fn text_selection(&self, key: &str) -> Option<(usize, usize)> {
        self.state.editors.get(key).map(|e| (e.anchor, e.cursor))
    }
    /// Forward input after preparing the view. Coordinates match the last prepared geometry.
    pub fn handle_event(&mut self, event: InputEvent) -> Vec<UiEvent> {
        let Some(scene) = self.input_scene.clone() else {
            return Vec::new();
        };
        self.handle_input(scene, event)
    }
    /// Use this when a prepared view has been translated by the host for presentation.
    pub fn handle_event_in(&mut self, prepared: &PreparedView, event: InputEvent) -> Vec<UiEvent> {
        if prepared.namespace != self.key
            || prepared.content_revision != self.content_revision
            || self.input_scene.is_none()
        {
            return Vec::new();
        }
        self.handle_input(InputScene::from(prepared), event)
    }
    fn handle_input(&mut self, scene: InputScene, event: InputEvent) -> Vec<UiEvent> {
        self.sync_state();
        let before = self.state.visual_fingerprint();
        let mut out = Vec::new();
        if !self.state.window_focused && !matches!(event, InputEvent::WindowFocus(_)) {
            return out;
        }
        let hit = |position: Point| {
            scene
                .controls
                .iter()
                .rev()
                .find(|c| !c.disabled && c.rect.contains(position) && c.clip.contains(position))
        };
        match event {
            InputEvent::WindowFocus(value) => {
                self.state.window_focused = value;
                if !value {
                    self.state.pressed = None;
                    self.state.dragging = false;
                    self.state.pending_paste = None;
                    for editor in self.state.editors.values_mut() {
                        editor.preedit = None;
                    }
                }
            }
            InputEvent::PointerDown { position, shift } => {
                let control = hit(position);
                self.change_focus(control.map(|c| c.key.clone()), &scene, &mut out);
                self.state.pressed = control.map(|c| c.key.clone());
                self.state.dragging = false;
                if let Some(control) = control
                    && matches!(control.kind, ControlKind::TextInput { .. })
                {
                    let editor = self.state.editors.get_mut(&control.key).unwrap();
                    let cursor = control
                        .line
                        .as_ref()
                        .map_or(0, |line| line.hit(position.x - control.text_origin.x));
                    editor.set_selection(if shift { editor.anchor } else { cursor }, cursor);
                    self.state.dragging = true;
                }
            }
            InputEvent::PointerMove { position } => {
                self.state.hover = hit(position).map(|c| c.key.clone());
                if self.state.dragging
                    && let Some(key) = &self.state.pressed
                    && let Some(control) = scene.controls.iter().find(|c| &c.key == key)
                    && let Some(editor) = self.state.editors.get_mut(key)
                {
                    let cursor = control
                        .line
                        .as_ref()
                        .map_or(0, |line| line.hit(position.x - control.text_origin.x));
                    editor.set_selection(editor.anchor, cursor);
                }
            }
            InputEvent::PointerUp { position } => {
                if !self.state.dragging
                    && let Some(control) = hit(position)
                    && self.state.pressed.as_deref() == Some(control.key.as_str())
                    && let ControlKind::Button(action) = &control.kind
                {
                    out.push(UiEvent::Activate(action.clone()));
                }
                self.state.pressed = None;
                self.state.dragging = false;
            }
            InputEvent::PointerCancel => {
                self.state.pressed = None;
                self.state.dragging = false;
            }
            InputEvent::Wheel { position, delta } => {
                if delta.x.is_finite() && delta.y.is_finite() {
                    // Deepest viewport first; unconsumed movement bubbles to its ancestors.
                    let mut remaining = delta;
                    for scroll in scene
                        .scrolls
                        .iter()
                        .rev()
                        .filter(|s| s.viewport.contains(position) && s.clip.contains(position))
                    {
                        let offset = self.state.scroll.entry(scroll.key.clone()).or_default();
                        let before = *offset;
                        offset.x = (offset.x + remaining.x).clamp(
                            0.0,
                            (scroll.extent.width - scroll.viewport.size.width).max(0.0),
                        );
                        offset.y = (offset.y + remaining.y).clamp(
                            0.0,
                            (scroll.extent.height - scroll.viewport.size.height).max(0.0),
                        );
                        remaining.x -= offset.x - before.x;
                        remaining.y -= offset.y - before.y;
                    }
                }
            }
            InputEvent::KeyDown {
                key: Key::Tab,
                modifiers,
            } => {
                if !modifiers.control && !modifiers.alt {
                    let controls: Vec<_> = scene.controls.iter().filter(|c| !c.disabled).collect();
                    if !controls.is_empty() {
                        let current = controls
                            .iter()
                            .position(|c| self.state.focus.as_deref() == Some(c.key.as_str()));
                        let index = match (current, modifiers.shift) {
                            (Some(0), true) | (None, true) => controls.len() - 1,
                            (Some(i), true) => i - 1,
                            (Some(i), false) => (i + 1) % controls.len(),
                            (None, false) => 0,
                        };
                        self.change_focus(Some(controls[index].key.clone()), &scene, &mut out);
                    }
                }
            }
            InputEvent::KeyDown {
                key: Key::Escape, ..
            } => {
                if let Some(editor) = self
                    .state
                    .focus
                    .as_ref()
                    .and_then(|key| self.state.editors.get_mut(key))
                    && editor.preedit.is_some()
                {
                    editor.preedit = None;
                } else {
                    out.push(UiEvent::Cancel);
                }
            }
            InputEvent::KeyDown { key, modifiers } => {
                if let Some(control) = scene
                    .controls
                    .iter()
                    .find(|c| !c.disabled && self.state.focus.as_deref() == Some(c.key.as_str()))
                {
                    match &control.kind {
                        ControlKind::Button(action) if !modifiers.control && !modifiers.alt => {
                            match key {
                                Key::Enter => out.push(UiEvent::Activate(action.clone())),
                                Key::Space => self.state.pressed = Some(control.key.clone()),
                                _ => {}
                            }
                        }
                        ControlKind::TextInput { read_only } => {
                            self.edit_key(&control.key, key, modifiers, *read_only, &mut out)
                        }
                        _ => {}
                    }
                }
            }
            InputEvent::KeyUp { key: Key::Space } => {
                if let Some(control) = scene.controls.iter().find(|c| {
                    !c.disabled
                        && self.state.focus.as_deref() == Some(c.key.as_str())
                        && self.state.pressed.as_deref() == Some(c.key.as_str())
                }) && let ControlKind::Button(action) = &control.kind
                {
                    out.push(UiEvent::Activate(action.clone()));
                }
                self.state.pressed = None;
            }
            InputEvent::KeyUp { .. } => {}
            InputEvent::Text(text) => {
                if text.is_empty() {
                    return out;
                }
                if let Some(control) = scene.controls.iter().find(|c| {
                    !c.disabled
                        && self.state.focus.as_deref() == Some(c.key.as_str())
                        && matches!(c.kind, ControlKind::TextInput { read_only: false })
                }) {
                    let editor = self.state.editors.get_mut(&control.key).unwrap();
                    let before = editor.value.clone();
                    editor.insert(&text);
                    if editor.value != before {
                        out.push(UiEvent::TextChanged {
                            key: control.key.clone(),
                            value: editor.value.clone(),
                        });
                    }
                }
            }
            InputEvent::Preedit { text, cursor } => {
                if let Some(control) = scene.controls.iter().find(|c| {
                    !c.disabled
                        && self.state.focus.as_deref() == Some(c.key.as_str())
                        && matches!(c.kind, ControlKind::TextInput { read_only: false })
                }) {
                    let editor = self.state.editors.get_mut(&control.key).unwrap();
                    let text = crate::input::single_line(&text);
                    editor.preedit = if text.is_empty() {
                        None
                    } else {
                        Some((
                            text.clone(),
                            cursor.map(|i| crate::input::boundary(&text, i)),
                        ))
                    };
                    editor.revision = editor.revision.wrapping_add(1);
                }
            }
            InputEvent::DeleteSurrounding { before, after } => {
                if let Some(control) = scene.controls.iter().find(|c| {
                    !c.disabled
                        && self.state.focus.as_deref() == Some(c.key.as_str())
                        && matches!(c.kind, ControlKind::TextInput { read_only: false })
                }) {
                    let editor = self.state.editors.get_mut(&control.key).unwrap();
                    let value = editor.value.clone();
                    let start =
                        crate::input::boundary(&editor.value, editor.cursor.saturating_sub(before));
                    let end = crate::input::boundary_after(
                        &editor.value,
                        editor.cursor.saturating_add(after).min(editor.value.len()),
                    );
                    editor.set_selection(start, end);
                    editor.insert("");
                    if value != editor.value {
                        out.push(UiEvent::TextChanged {
                            key: control.key.clone(),
                            value: editor.value.clone(),
                        });
                    }
                }
            }
            InputEvent::Paste { request, text } => {
                if self
                    .state
                    .pending_paste
                    .as_ref()
                    .is_some_and(|(serial, _, _)| *serial != request)
                {
                    return out;
                }
                if let Some((serial, key, revision)) = self.state.pending_paste.take()
                    && serial == request
                    && self.state.focus.as_ref() == Some(&key)
                    && let Some(editor) = self.state.editors.get_mut(&key)
                    && editor.revision == revision
                    && scene.controls.iter().any(|c| {
                        c.key == key
                            && !c.disabled
                            && matches!(c.kind, ControlKind::TextInput { read_only: false })
                    })
                {
                    let before = editor.value.clone();
                    editor.insert(&text);
                    if before != editor.value {
                        out.push(UiEvent::TextChanged {
                            key,
                            value: editor.value.clone(),
                        });
                    }
                }
            }
        }
        if before != self.state.visual_fingerprint() {
            self.invalidate();
        }
        out
    }
    fn change_focus(&mut self, key: Option<String>, scene: &InputScene, out: &mut Vec<UiEvent>) {
        if self.state.focus != key {
            if let Some(editor) = self
                .state
                .focus
                .as_ref()
                .and_then(|key| self.state.editors.get_mut(key))
            {
                editor.preedit = None;
            }
            self.state.focus = key;
            self.state.pending_paste = None;
            self.state.pressed = None;
            out.push(UiEvent::FocusChanged(self.state.focus.clone()));
        }
        if let Some(key) = &self.state.focus {
            self.reveal_key(key.clone(), scene);
        }
    }
    fn reveal_key(&mut self, key: String, scene: &InputScene) {
        if let Some(control) = scene.controls.iter().find(|c| c.key == key) {
            let mut target = control.rect;
            for ancestor in control.scroll_ancestors.iter().rev() {
                if let Some(scroll) = scene.scrolls.iter().find(|s| &s.key == ancestor) {
                    let offset = self.state.scroll.entry(ancestor.clone()).or_default();
                    let before = *offset;
                    let reveal = |start: f32, size: f32, v_start: f32, v_size: f32| {
                        if start < v_start {
                            start - v_start
                        } else if start + size > v_start + v_size {
                            (start + size - v_start - v_size).min(start - v_start)
                        } else {
                            0.0
                        }
                    };
                    offset.x = (offset.x
                        + reveal(
                            target.origin.x,
                            target.size.width,
                            scroll.viewport.origin.x,
                            scroll.viewport.size.width,
                        ))
                    .clamp(
                        0.0,
                        (scroll.extent.width - scroll.viewport.size.width).max(0.0),
                    );
                    offset.y = (offset.y
                        + reveal(
                            target.origin.y,
                            target.size.height,
                            scroll.viewport.origin.y,
                            scroll.viewport.size.height,
                        ))
                    .clamp(
                        0.0,
                        (scroll.extent.height - scroll.viewport.size.height).max(0.0),
                    );
                    target =
                        target.translated(Point::new(before.x - offset.x, before.y - offset.y));
                }
            }
        }
    }
    fn edit_key(
        &mut self,
        key: &str,
        command: Key,
        modifiers: Modifiers,
        read_only: bool,
        out: &mut Vec<UiEvent>,
    ) {
        let editor = self.state.editors.get_mut(key).unwrap();
        let before = editor.value.clone();
        if editor.preedit.is_some() {
            return;
        } // IME owns navigation while composing.
        match command {
            Key::Left | Key::Right | Key::Home | Key::End if !modifiers.alt => {
                editor.move_cursor(command, modifiers)
            }
            Key::A if modifiers.control => editor.set_selection(0, editor.value.len()),
            Key::C | Key::X if modifiers.control => {
                let selected = editor.value[editor.selection()].to_owned();
                if !selected.is_empty() {
                    out.push(UiEvent::ClipboardWrite(selected));
                    if command == Key::X && !read_only {
                        editor.insert("");
                    }
                }
            }
            Key::V if modifiers.control && !read_only => {
                self.state.paste_serial = self.state.paste_serial.wrapping_add(1);
                let request = self.state.paste_serial;
                self.state.pending_paste = Some((request, key.into(), editor.revision));
                out.push(UiEvent::ClipboardRead { request });
            }
            Key::Backspace | Key::Delete if !read_only && !modifiers.alt => {
                editor.delete(command == Key::Backspace, modifiers.control)
            }
            Key::Enter => out.push(UiEvent::Submit {
                key: key.into(),
                value: editor.value.clone(),
            }),
            _ => {}
        }
        if before != editor.value {
            out.push(UiEvent::TextChanged {
                key: key.into(),
                value: editor.value.clone(),
            });
        }
    }
    /// Call after each preparation to synchronize IME state and its candidate-popup anchor.
    pub fn ime_state(&self) -> UiEvent {
        self.ime_for_scene(self.input_scene.as_ref())
    }
    /// IME geometry for a translated presentation snapshot.
    pub fn ime_state_in(&self, prepared: &PreparedView) -> UiEvent {
        if prepared.namespace != self.key || prepared.content_revision != self.content_revision {
            return self.ime_for_scene(None);
        }
        self.ime_for_scene(Some(&InputScene::from(prepared)))
    }
    fn ime_for_scene(&self, scene: Option<&InputScene>) -> UiEvent {
        if self.state.window_focused
            && let Some(scene) = scene
            && let Some(control) = scene.controls.iter().find(|c| {
                !c.disabled
                    && self.state.focus.as_deref() == Some(c.key.as_str())
                    && matches!(c.kind, ControlKind::TextInput { read_only: false })
            })
            && let Some(editor) = self.state.editors.get(&control.key)
        {
            return UiEvent::Ime {
                key: Some(control.key.clone()),
                surrounding: editor.value.clone(),
                cursor: editor.cursor,
                anchor: editor.anchor,
                caret: control.caret.intersection(control.clip),
            };
        }
        UiEvent::Ime {
            key: None,
            surrounding: String::new(),
            cursor: 0,
            anchor: 0,
            caret: Rect::default(),
        }
    }
}

#[cfg(feature = "accessibility")]
impl UiView {
    /// Apply requests on the host/UI thread, then prepare and publish the new tree.
    pub fn handle_accessibility(
        &mut self,
        action: crate::accessibility::AccessibleAction,
    ) -> Vec<UiEvent> {
        use crate::accessibility::AccessibleAction;
        self.sync_state();
        let Some(scene) = self.input_scene.clone() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        match action {
            AccessibleAction::Focus(key) => {
                if scene.controls.iter().any(|c| c.key == key && !c.disabled) {
                    self.change_focus(Some(key), &scene, &mut out);
                }
            }
            AccessibleAction::Activate(key) => {
                if let Some(control) = scene.controls.iter().find(|c| c.key == key && !c.disabled)
                    && let ControlKind::Button(action) = &control.kind
                {
                    out.push(UiEvent::Activate(action.clone()));
                }
            }
            AccessibleAction::SetValue { key, value } => {
                if scene.controls.iter().any(|c| {
                    c.key == key
                        && !c.disabled
                        && matches!(c.kind, ControlKind::TextInput { read_only: false })
                }) {
                    let editor = self.state.editors.get_mut(&key).unwrap();
                    let before = editor.value.clone();
                    editor.set_selection(0, editor.value.len());
                    editor.insert(&value);
                    if before != editor.value {
                        out.push(UiEvent::TextChanged {
                            key,
                            value: editor.value.clone(),
                        });
                    }
                }
            }
            AccessibleAction::SetSelection {
                key,
                anchor,
                cursor,
            } => {
                if scene.controls.iter().any(|c| {
                    c.key == key && !c.disabled && matches!(c.kind, ControlKind::TextInput { .. })
                }) {
                    self.state
                        .editors
                        .get_mut(&key)
                        .unwrap()
                        .set_selection(anchor, cursor);
                }
            }
            AccessibleAction::Scroll { key, x, y } => {
                if x.is_finite()
                    && y.is_finite()
                    && let Some(scroll) = scene.scrolls.iter().find(|s| s.key == key)
                {
                    self.state.scroll.insert(
                        key,
                        Point::new(
                            x.clamp(
                                0.0,
                                (scroll.extent.width - scroll.viewport.size.width).max(0.0),
                            ),
                            y.clamp(
                                0.0,
                                (scroll.extent.height - scroll.viewport.size.height).max(0.0),
                            ),
                        ),
                    );
                }
            }
            AccessibleAction::Reveal(key) => {
                self.reveal_key(key, &scene);
            }
        }
        self.invalidate();
        out
    }
}
