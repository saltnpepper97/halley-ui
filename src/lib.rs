#![doc = include_str!("../README.md")]
//! UI building blocks shared by the Halley compositor and companion applications.
//!
//! Layout, text, and software rendering work without a Wayland connection or GPU.
//! The host retains its renderer, Wayland connection, buffers, and event loop.

#[cfg(feature = "accessibility")]
pub mod accessibility;
pub mod assets;
pub mod geometry;
pub mod input;
pub mod layout;
pub mod software;
pub mod text;
pub mod ui;

pub use geometry::{Color, Insets, Point, Rect, Size};
pub use text::{Font, TextOverflow, TextSystem};
pub use ui::{
    ActionId, Anchor, Button, Card, Column, Container, Image, Label, Row, Scroll, TextInput, Theme,
    UiView, Widget,
};

pub use input::{InputEvent, Key, Modifiers, UiEvent};
