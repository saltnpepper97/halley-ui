#![doc = include_str!("../docs/input.md")]
//! Host-neutral input. The host translates events and performs clipboard/IME requests.
use crate::{Point, Rect, ui::ActionId};
use std::{collections::HashMap, ops::Range};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    Tab,
    Enter,
    Space,
    Escape,
    Left,
    Right,
    Home,
    End,
    Backspace,
    Delete,
    A,
    C,
    X,
    V,
}
#[derive(Clone, Debug)]
pub enum InputEvent {
    PointerDown {
        position: Point,
        shift: bool,
    },
    PointerMove {
        position: Point,
    },
    PointerUp {
        position: Point,
    },
    PointerCancel,
    KeyDown {
        key: Key,
        modifiers: Modifiers,
    },
    KeyUp {
        key: Key,
    },
    /// Committed text from the host. Don't also insert it from a key event.
    Text(String),
    /// Temporary IME text and a UTF-8 byte cursor; None clears composition.
    Preedit {
        text: String,
        cursor: Option<usize>,
    },
    /// Delete surrounding committed text by UTF-8 byte counts, rounded to graphemes.
    DeleteSurrounding {
        before: usize,
        after: usize,
    },
    /// A reply to ClipboardRead. Stale replies are ignored.
    Paste {
        request: u64,
        text: String,
    },
    /// Positive deltas move the viewport towards the end, in UI pixels.
    Wheel {
        position: Point,
        delta: Point,
    },
    WindowFocus(bool),
}
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    Activate(ActionId),
    TextChanged {
        key: String,
        value: String,
    },
    Submit {
        key: String,
        value: String,
    },
    FocusChanged(Option<String>),
    ClipboardWrite(String),
    ClipboardRead {
        request: u64,
    },
    /// Host enables/disables its input method and sets the candidate popup rectangle.
    Ime {
        key: Option<String>,
        surrounding: String,
        cursor: usize,
        anchor: usize,
        caret: Rect,
    },
    Cancel,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Editor {
    pub value: String,
    pub anchor: usize,
    pub cursor: usize,
    pub preedit: Option<(String, Option<usize>)>,
    pub offset: f32,
    pub revision: u64,
}
impl Editor {
    pub fn new(value: &str) -> Self {
        let value = single_line(value);
        let cursor = value.len();
        Self {
            value,
            cursor,
            anchor: cursor,
            ..Default::default()
        }
    }
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    pub fn set_selection(&mut self, anchor: usize, cursor: usize) {
        self.anchor = boundary(&self.value, anchor);
        self.cursor = boundary(&self.value, cursor);
        self.preedit = None;
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn insert(&mut self, text: &str) {
        let text = single_line(text);
        let range = self.selection();
        self.value.replace_range(range.clone(), &text);
        self.cursor = boundary_after(&self.value, range.start + text.len());
        self.anchor = self.cursor;
        self.preedit = None;
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn display(&self) -> (String, usize) {
        if let Some((preedit, cursor)) = &self.preedit {
            let range = self.selection();
            let mut value = self.value.clone();
            value.replace_range(range.clone(), preedit);
            (
                value,
                range.start + boundary(preedit, cursor.unwrap_or(preedit.len())),
            )
        } else {
            (self.value.clone(), self.cursor)
        }
    }
    pub fn move_cursor(&mut self, key: Key, modifiers: Modifiers) {
        let selection = self.selection();
        let destination = match key {
            Key::Home => 0,
            Key::End => self.value.len(),
            Key::Left if !modifiers.shift && !selection.is_empty() => selection.start,
            Key::Right if !modifiers.shift && !selection.is_empty() => selection.end,
            Key::Left if modifiers.control => previous_word(&self.value, self.cursor),
            Key::Right if modifiers.control => next_word(&self.value, self.cursor),
            Key::Left => previous(&self.value, self.cursor),
            Key::Right => next(&self.value, self.cursor),
            _ => self.cursor,
        };
        self.set_selection(
            if modifiers.shift {
                self.anchor
            } else {
                destination
            },
            destination,
        );
    }
    pub fn delete(&mut self, backwards: bool, word: bool) {
        if self.selection().is_empty() {
            self.anchor = match (backwards, word) {
                (true, true) => previous_word(&self.value, self.cursor),
                (false, true) => next_word(&self.value, self.cursor),
                (true, false) => previous(&self.value, self.cursor),
                (false, false) => next(&self.value, self.cursor),
            };
        }
        self.insert("");
    }
}
pub(crate) fn single_line(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() && *c != '\u{2028}' && *c != '\u{2029}')
        .collect()
}
pub(crate) fn boundary(text: &str, index: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .take_while(|i| *i <= index)
        .last()
        .unwrap_or(0)
}
pub(crate) fn boundary_after(text: &str, index: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .find(|i| *i >= index)
        .unwrap_or(text.len())
}
fn previous(text: &str, cursor: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|i| *i < cursor)
        .last()
        .unwrap_or(0)
}
fn next(text: &str, cursor: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .find(|i| *i > cursor)
        .unwrap_or(text.len())
}
fn previous_word(text: &str, cursor: usize) -> usize {
    text.unicode_word_indices()
        .map(|(i, _)| i)
        .take_while(|i| *i < cursor)
        .last()
        .unwrap_or(0)
}
fn next_word(text: &str, cursor: usize) -> usize {
    text.unicode_word_indices()
        .map(|(i, s)| i + s.len())
        .find(|i| *i > cursor)
        .unwrap_or(text.len())
}
#[derive(Default)]
pub(crate) struct State {
    pub editors: HashMap<String, Editor>,
    pub scroll: HashMap<String, Point>,
    pub focus: Option<String>,
    pub hover: Option<String>,
    pub pressed: Option<String>,
    pub dragging: bool,
    pub window_focused: bool,
    pub paste_serial: u64,
    pub pending_paste: Option<(u64, String, u64)>,
}

impl State {
    pub fn visual_fingerprint(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hash = DefaultHasher::new();
        self.focus.hash(&mut hash);
        self.hover.hash(&mut hash);
        self.pressed.hash(&mut hash);
        self.window_focused.hash(&mut hash);
        // Order-independent map summaries; retained hash-map iteration does not change during input.
        let mut editors = 0u64;
        for (key, editor) in &self.editors {
            let mut h = DefaultHasher::new();
            key.hash(&mut h);
            editor.revision.hash(&mut h);
            editor.preedit.is_some().hash(&mut h);
            editors ^= h.finish();
        }
        editors.hash(&mut hash);
        let mut scrolls = 0u64;
        for (key, p) in &self.scroll {
            let mut h = DefaultHasher::new();
            key.hash(&mut h);
            p.x.to_bits().hash(&mut h);
            p.y.to_bits().hash(&mut h);
            scrolls ^= h.finish();
        }
        scrolls.hash(&mut hash);
        hash.finish()
    }
}
