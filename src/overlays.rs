#![doc = include_str!("../docs/components/overlays.md")]
//! Renderer-neutral popovers, menus, modal dialogs, and input/focus scoping.
use super::*;
use crate::{InputEvent, Key, UiEvent};

#[path = "overlay_stack.rs"]
mod stack;
pub use stack::UiStack;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerId {
    Base,
    Overlay(String),
}
impl LayerId {
    pub fn overlay(key: impl Into<String>) -> Self {
        Self::Overlay(key.into())
    }
    fn prefix(&self) -> String {
        match self {
            Self::Base => "base/".into(),
            Self::Overlay(key) => format!("overlay/{}:{key}/", key.len()),
        }
    }
    fn qualify(&self, key: &str) -> String {
        format!("{}{key}", self.prefix())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OverlayAnchor {
    Widget { layer: LayerId, key: String },
    Rect(Rect),
}
impl OverlayAnchor {
    pub fn base(key: impl Into<String>) -> Self {
        Self::Widget {
            layer: LayerId::Base,
            key: key.into(),
        }
    }
    pub fn overlay(layer: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Widget {
            layer: LayerId::overlay(layer),
            key: key.into(),
        }
    }
    pub fn rect(rect: Rect) -> Self {
        Self::Rect(rect)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    #[default]
    BelowStart,
    BelowEnd,
    AboveStart,
    AboveEnd,
    RightStart,
    RightEnd,
    LeftStart,
    LeftEnd,
    Center,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayKind {
    Popover,
    Menu,
    Dialog,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Programmatic,
    Escape,
    OutsidePress,
    Activated,
    AnchorLost,
    WindowBlur,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayerEvent {
    View { layer: LayerId, event: UiEvent },
    Opened { key: String },
    Closed { key: String, reason: CloseReason },
}

#[derive(Debug, thiserror::Error)]
pub enum OverlayError {
    #[error("prepare the stack before opening an overlay or routing geometry-dependent input")]
    NotPrepared,
    #[error("overlay already open: {0}")]
    DuplicateKey(String),
    #[error("unknown overlay: {0}")]
    UnknownOverlay(String),
    #[error("unknown or clipped anchor {key} in layer {layer:?}")]
    UnknownAnchor { layer: LayerId, key: String },
    #[error("layer does not own input: {0:?}")]
    InactiveLayer(LayerId),
    #[error("overlay dimensions and offsets must be finite and nonnegative")]
    InvalidGeometry,
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error(transparent)]
    Interaction(#[from] InteractionError),
}

/// Unset surfaces inherit `Theme::card`; unset padding uses the spacing defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayTheme {
    pub popover: Option<CardStyle>,
    pub menu: Option<CardStyle>,
    pub dialog: Option<CardStyle>,
    pub padding: Option<Insets>,
    pub backdrop: Color,
}
impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            popover: None,
            menu: None,
            dialog: None,
            padding: None,
            backdrop: Color::rgba(0.0, 0.0, 0.0, 0.35),
        }
    }
}
impl OverlayTheme {
    pub(super) fn validate(&self) -> Result<(), LayoutError> {
        let color_valid = |c: Color| [c.r, c.g, c.b, c.a].into_iter().all(f32::is_finite);
        let mut valid = color_valid(self.backdrop);
        for style in [self.popover, self.menu, self.dialog].into_iter().flatten() {
            valid &= color_valid(style.fill)
                && color_valid(style.border)
                && [style.border_width, style.radius]
                    .into_iter()
                    .all(nonnegative);
        }
        if let Some(padding) = self.padding {
            valid &= [padding.left, padding.right, padding.top, padding.bottom]
                .into_iter()
                .all(nonnegative);
        }
        if valid {
            Ok(())
        } else {
            Err(LayoutError::InvalidStyle("overlay theme".into()))
        }
    }
}

/// A definition can be updated while its open view retains editor/list state.
#[derive(Clone, Debug)]
pub struct Overlay {
    key: String,
    kind: OverlayKind,
    anchor: Option<OverlayAnchor>,
    placement: Placement,
    offset: f32,
    width: Option<f32>,
    height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    padding: Option<Insets>,
    style: Option<CardStyle>,
    title: Option<String>,
    initial_focus: Option<String>,
    dismiss_outside: bool,
    dismiss_escape: bool,
    dismiss_blur: bool,
    close_on_activate: bool,
    children: Vec<Widget>,
    items: Vec<ListItem>,
    selected: Option<String>,
    item_style: Option<StyleSheet>,
}
impl Overlay {
    fn new(key: impl Into<String>, kind: OverlayKind, anchor: Option<OverlayAnchor>) -> Self {
        let modal = kind == OverlayKind::Dialog;
        Self {
            key: key.into(),
            kind,
            anchor,
            placement: if modal {
                Placement::Center
            } else {
                Placement::BelowStart
            },
            offset: 4.0,
            width: match kind {
                OverlayKind::Dialog => Some(360.0),
                OverlayKind::Menu => Some(240.0),
                _ => None,
            },
            height: None,
            max_width: None,
            max_height: None,
            padding: None,
            style: None,
            title: None,
            initial_focus: None,
            dismiss_outside: !modal,
            dismiss_escape: true,
            dismiss_blur: !modal,
            close_on_activate: kind == OverlayKind::Menu,
            children: Vec::new(),
            items: Vec::new(),
            selected: None,
            item_style: None,
        }
    }
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn kind(&self) -> OverlayKind {
        self.kind
    }
    fn validate(&self) -> Result<(), OverlayError> {
        let dims_valid = [
            Some(self.offset),
            self.width,
            self.height,
            self.max_width,
            self.max_height,
        ]
        .into_iter()
        .flatten()
        .all(nonnegative);
        let padding_valid = self.padding.is_none_or(|p| {
            [p.left, p.right, p.top, p.bottom]
                .into_iter()
                .all(nonnegative)
        });
        let anchor_valid = match self.anchor {
            Some(OverlayAnchor::Rect(r)) => rect_valid(r),
            _ => true,
        };
        if dims_valid && padding_valid && anchor_valid {
            Ok(())
        } else {
            Err(OverlayError::InvalidGeometry)
        }
    }
    fn widget(&self, theme: &Theme, available: Size) -> Widget {
        let key = &self.key;
        let themed = match self.kind {
            OverlayKind::Popover => theme.overlays.popover,
            OverlayKind::Menu => theme.overlays.menu,
            OverlayKind::Dialog => theme.overlays.dialog,
        };
        let mut padding = self.padding.or(theme.overlays.padding).unwrap_or_else(|| {
            Insets::all(if self.kind == OverlayKind::Menu {
                theme.spacing.small
            } else {
                theme.spacing.large
            })
        });
        // Even chrome must fit a tiny viewport or an explicitly small surface.
        let width = available
            .width
            .min(self.width.unwrap_or(f32::INFINITY))
            .min(self.max_width.unwrap_or(f32::INFINITY));
        let height = available
            .height
            .min(self.height.unwrap_or(f32::INFINITY))
            .min(self.max_height.unwrap_or(f32::INFINITY));
        let x = (width / (padding.left + padding.right)).min(1.0);
        let y = (height / (padding.top + padding.bottom)).min(1.0);
        padding.left *= x;
        padding.right *= x;
        padding.top *= y;
        padding.bottom *= y;
        let mut card =
            Card::new(format!("{key}/surface")).style(self.style.or(themed).unwrap_or(theme.card));
        // Explicit insets are supported internally; the public Card builders remain unchanged.
        card.0.layout.padding = Some(padding);
        card.0.layout.width = self.width;
        card.0.layout.height = self.height;
        card.0.layout.max_width = self.max_width;
        card.0.layout.max_height = self.max_height;
        card.0.accessible_label = self.title.clone();
        if self.kind == OverlayKind::Menu {
            let mut list = List::new(key).gap(theme.spacing.list_gap);
            if let Some(style) = &self.item_style {
                list = list.item_style(style.clone());
            }
            let first = self.items.iter().find_map(|item| {
                let widget: Widget = item.clone().into();
                matches!(
                    widget.kind,
                    Kind::ListItem {
                        disabled: false,
                        ..
                    }
                )
                .then_some(widget.key)
            });
            if let Some(selected) = self.selected.clone().or(first) {
                list = list.selected(selected);
            }
            for item in &self.items {
                list = list.child(item.clone());
            }
            card = card.child(list);
        } else {
            let mut body = Column::new(format!("{key}/body")).gap(theme.spacing.medium);
            for child in &self.children {
                body = body.child(child.clone());
            }
            card = card.child(Scroll::new(format!("{key}/viewport")).child(body));
        }
        card.into()
    }
}

macro_rules! options {
    () => {
        pub fn width(mut self, value: f32) -> Self {
            self.0.width = Some(value);
            self
        }
        pub fn height(mut self, value: f32) -> Self {
            self.0.height = Some(value);
            self
        }
        pub fn max_width(mut self, value: f32) -> Self {
            self.0.max_width = Some(value);
            self
        }
        pub fn max_height(mut self, value: f32) -> Self {
            self.0.max_height = Some(value);
            self
        }
        pub fn padding(mut self, value: f32) -> Self {
            self.0.padding = Some(Insets::all(value));
            self
        }
        pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
            self.0.padding = Some(Insets::xy(x, y));
            self
        }
        pub fn style(mut self, value: CardStyle) -> Self {
            self.0.style = Some(value);
            self
        }
        pub fn accessible_label(mut self, value: impl Into<String>) -> Self {
            self.0.title = Some(value.into());
            self
        }
        pub fn initial_focus(mut self, key: impl Into<String>) -> Self {
            self.0.initial_focus = Some(key.into());
            self
        }
        pub fn dismiss_on_outside(mut self, value: bool) -> Self {
            self.0.dismiss_outside = value;
            self
        }
        pub fn dismiss_on_escape(mut self, value: bool) -> Self {
            self.0.dismiss_escape = value;
            self
        }
        pub fn dismiss_on_blur(mut self, value: bool) -> Self {
            self.0.dismiss_blur = value;
            self
        }
        pub fn close_on_activate(mut self, value: bool) -> Self {
            self.0.close_on_activate = value;
            self
        }
    };
}
macro_rules! anchored {
    () => {
        pub fn placement(mut self, value: Placement) -> Self {
            self.0.placement = value;
            self
        }
        pub fn offset(mut self, value: f32) -> Self {
            self.0.offset = value;
            self
        }
    };
}

#[derive(Clone, Debug)]
pub struct Popover(Overlay);
impl Popover {
    pub fn new(key: impl Into<String>, anchor: OverlayAnchor) -> Self {
        Self(Overlay::new(key, OverlayKind::Popover, Some(anchor)))
    }
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.0.children.push(child.into());
        self
    }
    options!();
    anchored!();
}
impl From<Popover> for Overlay {
    fn from(value: Popover) -> Self {
        value.0
    }
}

#[derive(Clone, Debug)]
pub struct Dialog(Overlay);
impl Dialog {
    pub fn new(key: impl Into<String>) -> Self {
        Self(Overlay::new(key, OverlayKind::Dialog, None))
    }
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.0.children.push(child.into());
        self
    }
    options!();
}
impl From<Dialog> for Overlay {
    fn from(value: Dialog) -> Self {
        value.0
    }
}

/// Menu items share the list option's label, action, disabled, and custom-content builders.
pub type MenuItem = ListItem;
#[derive(Clone, Debug)]
pub struct Menu(Overlay);
impl Menu {
    pub fn new(key: impl Into<String>, anchor: OverlayAnchor) -> Self {
        Self(Overlay::new(key, OverlayKind::Menu, Some(anchor)))
    }
    pub fn child(mut self, item: MenuItem) -> Self {
        self.0.items.push(item);
        self
    }
    pub fn selected(mut self, key: impl Into<String>) -> Self {
        self.0.selected = Some(key.into());
        self
    }
    pub fn item_style(mut self, style: StyleSheet) -> Self {
        self.0.item_style = Some(style);
        self
    }
    options!();
    anchored!();
}
impl From<Menu> for Overlay {
    fn from(value: Menu) -> Self {
        value.0
    }
}

fn nonnegative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}
fn rect_valid(rect: Rect) -> bool {
    rect.origin.x.is_finite()
        && rect.origin.y.is_finite()
        && nonnegative(rect.size.width)
        && nonnegative(rect.size.height)
}

fn place(
    anchor: Rect,
    size: Size,
    bounds: Rect,
    preferred: Placement,
    gap: f32,
) -> (Point, Placement) {
    let candidate = |placement| {
        let x = anchor.origin.x;
        let y = anchor.origin.y;
        let right = x + anchor.size.width;
        let bottom = y + anchor.size.height;
        match placement {
            Placement::BelowStart => Point::new(x, bottom + gap),
            Placement::BelowEnd => Point::new(right - size.width, bottom + gap),
            Placement::AboveStart => Point::new(x, y - size.height - gap),
            Placement::AboveEnd => Point::new(right - size.width, y - size.height - gap),
            Placement::RightStart => Point::new(right + gap, y),
            Placement::RightEnd => Point::new(right + gap, bottom - size.height),
            Placement::LeftStart => Point::new(x - size.width - gap, y),
            Placement::LeftEnd => Point::new(x - size.width - gap, bottom - size.height),
            Placement::Center => Point::new(
                bounds.origin.x + (bounds.size.width - size.width) * 0.5,
                bounds.origin.y + (bounds.size.height - size.height) * 0.5,
            ),
        }
    };
    let opposite = match preferred {
        Placement::BelowStart => Placement::AboveStart,
        Placement::BelowEnd => Placement::AboveEnd,
        Placement::AboveStart => Placement::BelowStart,
        Placement::AboveEnd => Placement::BelowEnd,
        Placement::RightStart => Placement::LeftStart,
        Placement::RightEnd => Placement::LeftEnd,
        Placement::LeftStart => Placement::RightStart,
        Placement::LeftEnd => Placement::RightEnd,
        Placement::Center => Placement::Center,
    };
    // Cross-axis overflow is clamped independently; it must not prevent a side flip.
    let fits = |p: Point| match preferred {
        Placement::BelowStart
        | Placement::BelowEnd
        | Placement::AboveStart
        | Placement::AboveEnd => {
            p.y >= bounds.origin.y && p.y + size.height <= bounds.origin.y + bounds.size.height
        }
        Placement::RightStart | Placement::RightEnd | Placement::LeftStart | Placement::LeftEnd => {
            p.x >= bounds.origin.x && p.x + size.width <= bounds.origin.x + bounds.size.width
        }
        Placement::Center => true,
    };
    let preferred_point = candidate(preferred);
    let (point, placement) = if !fits(preferred_point) && fits(candidate(opposite)) {
        (candidate(opposite), opposite)
    } else {
        (preferred_point, preferred)
    };
    (
        Point::new(
            point.x.clamp(
                bounds.origin.x,
                bounds.origin.x + (bounds.size.width - size.width).max(0.0),
            ),
            point.y.clamp(
                bounds.origin.y,
                bounds.origin.y + (bounds.size.height - size.height).max(0.0),
            ),
        ),
        placement,
    )
}
