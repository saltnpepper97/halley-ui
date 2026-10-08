#![doc = include_str!("../docs/components/text-input.md")]
//! Composable widgets and cached [Taffy](https://github.com/DioxusLabs/taffy) layout.
//! Paint and input use one prepared view.
use crate::{Color, Font, Insets, Point, Rect, Size, TextOverflow, TextSystem, assets::ImageData};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use taffy::{TaffyTree, prelude as tf};
#[path = "interaction.rs"]
mod interaction;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActionId(pub String);
impl ActionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Anchor {
    TopLeft,
    #[default]
    TopCenter,
    TopRight,
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
}
impl Anchor {
    pub fn position(self, bounds: Rect, size: Size, margin: f32) -> Point {
        let x = match self {
            Self::TopLeft | Self::BottomLeft => margin,
            Self::TopCenter | Self::Center | Self::BottomCenter => {
                (bounds.size.width - size.width) / 2.0
            }
            Self::TopRight | Self::BottomRight => bounds.size.width - size.width - margin,
        };
        let y = match self {
            Self::TopLeft | Self::TopCenter | Self::TopRight => margin,
            Self::Center => (bounds.size.height - size.height) / 2.0,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight => {
                bounds.size.height - size.height - margin
            }
        };
        Point::new(bounds.origin.x + x, bounds.origin.y + y)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardStyle {
    pub fill: Color,
    pub border: Color,
    pub border_width: f32,
    pub radius: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub font: Font,
    pub text: Color,
    pub subtext: Color,
    pub card: CardStyle,
    pub button: CardStyle,
    pub button_hover: Color,
    pub button_pressed: Color,
}
impl Default for Theme {
    fn default() -> Self {
        let fill = Color::rgb8(38, 46, 56);
        let border = Color::rgb8(214, 93, 38);
        Self {
            font: Font::default(),
            text: Color::rgb8(240, 245, 250),
            subtext: Color::rgb8(190, 195, 200),
            card: CardStyle {
                fill,
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
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Align {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct LayoutStyle {
    padding: Insets,
    gap: f32,
    width: Option<f32>,
    height: Option<f32>,
    max_width: Option<f32>,
    max_height: Option<f32>,
    grow: f32,
    align: Align,
}
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Card {
        style: Option<CardStyle>,
    },
    Row,
    Column,
    Label {
        text: String,
        color: Option<Color>,
        font: Option<Font>,
        overflow: TextOverflow,
        align: Align,
    },
    Button {
        text: String,
        action: ActionId,
        disabled: bool,
    },
    Image {
        data: Arc<ImageData>,
    },
    TextInput {
        initial: String,
        placeholder: String,
        disabled: bool,
        read_only: bool,
    },
    Scroll,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    key: String,
    kind: Kind,
    layout: LayoutStyle,
    children: Vec<Widget>,
    accessible_label: Option<String>,
}
impl Widget {
    fn new(key: impl Into<String>, kind: Kind) -> Self {
        Self {
            key: key.into(),
            kind,
            layout: LayoutStyle::default(),
            children: Vec::new(),
            accessible_label: None,
        }
    }
}
macro_rules! layout_methods {
    () => {
        /// Accessible name. Text inputs need an explicit label; placeholder is only a hint.
        pub fn accessible_label(mut self, value: impl Into<String>) -> Self {
            self.0.accessible_label = Some(value.into());
            self
        }
        pub fn padding(mut self, value: f32) -> Self {
            self.0.layout.padding = Insets::all(value.max(0.0));
            self
        }
        pub fn padding_xy(mut self, x: f32, y: f32) -> Self {
            self.0.layout.padding = Insets::xy(x.max(0.0), y.max(0.0));
            self
        }
        pub fn width(mut self, value: f32) -> Self {
            self.0.layout.width = Some(value.max(0.0));
            self
        }
        pub fn height(mut self, value: f32) -> Self {
            self.0.layout.height = Some(value.max(0.0));
            self
        }
        pub fn max_width(mut self, value: f32) -> Self {
            self.0.layout.max_width = Some(value.max(0.0));
            self
        }
        pub fn max_height(mut self, value: f32) -> Self {
            self.0.layout.max_height = Some(value.max(0.0));
            self
        }
        pub fn grow(mut self, value: f32) -> Self {
            self.0.layout.grow = value.max(0.0);
            self
        }
    };
}
macro_rules! container {
    ($name:ident, $kind:expr) => {
        #[derive(Clone, Debug)]
        pub struct $name(Widget);
        impl $name {
            pub fn new(key: impl Into<String>) -> Self {
                Self(Widget::new(key, $kind))
            }
            pub fn child(mut self, child: impl Into<Widget>) -> Self {
                self.0.children.push(child.into());
                self
            }
            pub fn gap(mut self, value: f32) -> Self {
                self.0.layout.gap = value.max(0.0);
                self
            }
            pub fn align(mut self, value: Align) -> Self {
                self.0.layout.align = value;
                self
            }
            layout_methods!();
        }
        impl From<$name> for Widget {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
container!(Card, Kind::Card { style: None });
container!(Row, Kind::Row);
container!(Column, Kind::Column);
container!(Container, Kind::Column);
impl Card {
    pub fn style(mut self, style: CardStyle) -> Self {
        self.0.kind = Kind::Card { style: Some(style) };
        self
    }
}

#[derive(Clone, Debug)]
pub struct Label(Widget);
impl Label {
    pub fn new(key: impl Into<String>, text: impl Into<String>) -> Self {
        Self(Widget::new(
            key,
            Kind::Label {
                text: text.into(),
                color: None,
                font: None,
                overflow: TextOverflow::Clip,
                align: Align::Start,
            },
        ))
    }
    pub fn color(mut self, value: Color) -> Self {
        if let Kind::Label { color, .. } = &mut self.0.kind {
            *color = Some(value);
        }
        self
    }
    pub fn font(mut self, value: Font) -> Self {
        if let Kind::Label { font, .. } = &mut self.0.kind {
            *font = Some(value);
        }
        self
    }
    pub fn overflow(mut self, value: TextOverflow) -> Self {
        if let Kind::Label { overflow, .. } = &mut self.0.kind {
            *overflow = value;
        }
        self
    }
    pub fn align(mut self, value: Align) -> Self {
        if let Kind::Label { align, .. } = &mut self.0.kind {
            *align = value;
        }
        self
    }
    layout_methods!();
}
impl From<Label> for Widget {
    fn from(value: Label) -> Self {
        value.0
    }
}

#[derive(Clone, Debug)]
pub struct Button(Widget);
impl Button {
    pub fn new(key: impl Into<String>, text: impl Into<String>) -> Self {
        let key = key.into();
        let mut widget = Widget::new(
            key.clone(),
            Kind::Button {
                text: text.into(),
                action: ActionId::new(key),
                disabled: false,
            },
        );
        widget.layout.padding = Insets::xy(12.0, 8.0);
        Self(widget)
    }
    /// Use custom content while keeping the button's focus and activation behavior.
    pub fn child(mut self, child: impl Into<Widget>) -> Self {
        self.0.children.push(child.into());
        self
    }
    pub fn action(mut self, value: ActionId) -> Self {
        if let Kind::Button { action, .. } = &mut self.0.kind {
            *action = value;
        }
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        if let Kind::Button { disabled, .. } = &mut self.0.kind {
            *disabled = value;
        }
        self
    }
    layout_methods!();
}
impl From<Button> for Widget {
    fn from(value: Button) -> Self {
        value.0
    }
}

/// A single-line editor. Its initial value seeds retained state once per key.
#[derive(Clone, Debug)]
pub struct TextInput(Widget);
impl TextInput {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut widget = Widget::new(
            key,
            Kind::TextInput {
                initial: crate::input::single_line(&value.into()),
                placeholder: String::new(),
                disabled: false,
                read_only: false,
            },
        );
        widget.layout.padding = Insets::xy(10.0, 8.0);
        widget.layout.width = Some(240.0);
        Self(widget)
    }
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        if let Kind::TextInput { placeholder, .. } = &mut self.0.kind {
            *placeholder = crate::input::single_line(&value.into());
        }
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        if let Kind::TextInput { disabled, .. } = &mut self.0.kind {
            *disabled = value;
        }
        self
    }
    pub fn read_only(mut self, value: bool) -> Self {
        if let Kind::TextInput { read_only, .. } = &mut self.0.kind {
            *read_only = value;
        }
        self
    }
    layout_methods!();
}
impl From<TextInput> for Widget {
    fn from(value: TextInput) -> Self {
        value.0
    }
}
container!(Scroll, Kind::Scroll);

/// Icons use this same component after decoding through `assets::ImageData`.
#[derive(Clone, Debug)]
pub struct Image(Widget);
impl Image {
    pub fn new(key: impl Into<String>, data: Arc<ImageData>) -> Self {
        Self(Widget::new(key, Kind::Image { data }))
    }
    layout_methods!();
}
impl From<Image> for Widget {
    fn from(value: Image) -> Self {
        value.0
    }
}

#[derive(Clone, Debug)]
pub enum PaintItem {
    Card {
        key: String,
        rect: Rect,
        clip: Rect,
        style: CardStyle,
    },
    Text {
        key: String,
        rect: Rect,
        clip: Rect,
        text: String,
        font: Font,
        color: Color,
    },
    Image {
        key: String,
        rect: Rect,
        clip: Rect,
        data: Arc<ImageData>,
    },
}
#[derive(Clone, Debug)]
pub struct HitRegion {
    pub key: String,
    pub rect: Rect,
    pub clip: Rect,
    pub action: ActionId,
    pub disabled: bool,
}
#[derive(Clone, Debug)]
pub enum ControlKind {
    Button(ActionId),
    TextInput { read_only: bool },
}
#[derive(Clone, Debug)]
pub struct ControlRegion {
    pub key: String,
    pub rect: Rect,
    pub clip: Rect,
    pub content: Rect,
    pub kind: ControlKind,
    pub disabled: bool,
    pub scroll_ancestors: Vec<String>,
    pub line: Option<Arc<crate::text::LineGeometry>>,
    pub text_origin: Point,
    pub caret: Rect,
}
#[derive(Clone, Debug)]
pub struct ScrollRegion {
    pub key: String,
    pub viewport: Rect,
    pub clip: Rect,
    pub extent: Size,
    pub offset: Point,
    pub ancestors: Vec<String>,
}
#[derive(Clone, Debug)]
pub enum SemanticRole {
    Group,
    Label,
    Button,
    TextInput,
    Image,
    Scroll,
}
#[derive(Clone, Debug)]
pub struct SemanticNode {
    pub key: String,
    pub role: SemanticRole,
    pub label: Option<String>,
    pub value: Option<String>,
    pub children: Vec<String>,
    pub rect: Rect,
    pub clip: Rect,
    pub disabled: bool,
    pub read_only: bool,
    pub selection: Option<(usize, usize)>,
}
#[derive(Clone, Debug, Default)]
pub struct PreparedView {
    pub items: Vec<PaintItem>,
    pub hits: Vec<HitRegion>,
    pub bounds: Rect,
    pub rects: HashMap<String, Rect>,
    pub controls: Vec<ControlRegion>,
    pub scrolls: Vec<ScrollRegion>,
    pub semantics: Vec<SemanticNode>,
    pub focused: Option<String>,
    pub namespace: String,
    pub content_revision: u64,
}
impl PreparedView {
    pub fn hit(&self, point: Point) -> Option<&HitRegion> {
        self.hits
            .iter()
            .rev()
            .find(|h| !h.disabled && h.rect.contains(point) && h.clip.contains(point))
    }
    /// Applies the same presentation transform to paint, clips, and input regions.
    pub fn translated(&self, offset: Point) -> Self {
        let mut view = self.clone();
        view.bounds = view.bounds.translated(offset);
        for item in &mut view.items {
            match item {
                PaintItem::Card { rect, clip, .. }
                | PaintItem::Image { rect, clip, .. }
                | PaintItem::Text { rect, clip, .. } => {
                    *rect = rect.translated(offset);
                    *clip = clip.translated(offset);
                }
            }
        }
        for hit in &mut view.hits {
            hit.rect = hit.rect.translated(offset);
            hit.clip = hit.clip.translated(offset);
        }
        for control in &mut view.controls {
            control.rect = control.rect.translated(offset);
            control.clip = control.clip.translated(offset);
            control.content = control.content.translated(offset);
            control.caret = control.caret.translated(offset);
            control.text_origin = Point::new(
                control.text_origin.x + offset.x,
                control.text_origin.y + offset.y,
            );
        }
        for scroll in &mut view.scrolls {
            scroll.viewport = scroll.viewport.translated(offset);
            scroll.clip = scroll.clip.translated(offset);
        }
        for node in &mut view.semantics {
            node.rect = node.rect.translated(offset);
            node.clip = node.clip.translated(offset);
        }
        for rect in view.rects.values_mut() {
            *rect = rect.translated(offset);
        }
        view
    }
    pub fn button_visuals(&mut self, theme: &Theme, hovered: Option<&str>, pressed: Option<&str>) {
        for hit in &self.hits {
            for item in &mut self.items {
                if let PaintItem::Card { key, style, .. } = item
                    && key == &hit.key
                {
                    *style = theme.button;
                    if hit.disabled {
                        style.fill.a *= 0.5;
                    } else if pressed == Some(key.as_str()) {
                        style.fill = theme.button_pressed;
                    } else if hovered == Some(key.as_str()) {
                        style.fill = theme.button_hover;
                    }
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("duplicate widget key: {0}")]
    DuplicateKey(String),
    #[error("layout bounds and style values must be finite and nonnegative")]
    InvalidGeometry,
    #[error("Taffy layout: {0}")]
    Taffy(#[from] taffy::TaffyError),
}
struct Cached {
    bounds: Rect,
    theme: Theme,
    prepared: PreparedView,
}
/// Keep a view alive between frames to reuse its layout. Replacing content invalidates it.
pub struct UiView {
    key: String,
    root: Widget,
    anchor: Anchor,
    margin: f32,
    cached: Option<Cached>,
    state: crate::input::State,
    editing_text: Option<TextSystem>,
    input_scene: Option<interaction::InputScene>,
    content_revision: u64,
}
impl UiView {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            root: Column::new("root").into(),
            anchor: Anchor::default(),
            margin: 0.0,
            cached: None,
            state: crate::input::State {
                window_focused: true,
                ..Default::default()
            },
            editing_text: None,
            input_scene: None,
            content_revision: 0,
        }
    }
    pub fn anchor(mut self, value: Anchor) -> Self {
        self.anchor = value;
        self.cached = None;
        self
    }
    pub fn margin(mut self, value: f32) -> Self {
        self.margin = value.max(0.0);
        self.cached = None;
        self
    }
    pub fn content(mut self, value: impl Into<Widget>) -> Self {
        self.root = value.into();
        self.cached = None;
        self
    }
    pub fn set_content(&mut self, value: impl Into<Widget>) {
        let value = value.into();
        if self.root != value {
            self.root = value;
            self.content_revision = self.content_revision.wrapping_add(1);
            self.cached = None;
            self.input_scene = None;
        }
    }
    /// Whether input/content changes require a new prepared snapshot.
    pub fn needs_prepare(&self) -> bool {
        self.cached.is_none()
    }
    pub fn invalidate(&mut self) {
        self.cached = None;
    }
    pub fn prepare(
        &mut self,
        bounds: Rect,
        theme: &Theme,
        text: &mut TextSystem,
    ) -> Result<&PreparedView, LayoutError> {
        self.prepare_with_color_measure(bounds, theme, |font, value, color| {
            text.raster(font, value, color.bytes())
                .map_or(Size::default(), |r| r.size())
        })
    }
    /// Hosts with an existing text/texture cache can supply its measurements directly.
    pub fn prepare_with_measure(
        &mut self,
        bounds: Rect,
        theme: &Theme,
        mut measure: impl FnMut(&Font, &str) -> Size,
    ) -> Result<&PreparedView, LayoutError> {
        self.prepare_with_color_measure(bounds, theme, |font, value, _| measure(font, value))
    }
    /// Color-aware measurements let rendering backends reuse the exact glyph texture.
    pub fn prepare_with_color_measure(
        &mut self,
        bounds: Rect,
        theme: &Theme,
        mut measure: impl FnMut(&Font, &str, Color) -> Size,
    ) -> Result<&PreparedView, LayoutError> {
        self.sync_state();
        if self
            .cached
            .as_ref()
            .is_some_and(|c| c.bounds == bounds && &c.theme == theme)
        {
            return Ok(&self.cached.as_ref().unwrap().prepared);
        }
        if ![
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
            self.margin,
        ]
        .iter()
        .all(|v| v.is_finite())
            || bounds.size.width < 0.0
            || bounds.size.height < 0.0
        {
            return Err(LayoutError::InvalidGeometry);
        }
        let mut tree = TaffyTree::<Kind>::new();
        tree.disable_rounding();
        let mut nodes = HashMap::new();
        let mut keys = HashSet::new();
        let root = build_tree(&mut tree, &self.root, &mut nodes, &mut keys, &self.state)?;
        let available = tf::Size {
            width: tf::AvailableSpace::Definite((bounds.size.width - self.margin * 2.0).max(0.0)),
            height: tf::AvailableSpace::Definite((bounds.size.height - self.margin * 2.0).max(0.0)),
        };
        let mut root_style = tree.style(root)?.clone();
        root_style.max_size.width = tf::length(
            (bounds.size.width - self.margin * 2.0)
                .max(0.0)
                .min(self.root.layout.max_width.unwrap_or(f32::INFINITY)),
        );
        root_style.max_size.height = tf::length(
            (bounds.size.height - self.margin * 2.0)
                .max(0.0)
                .min(self.root.layout.max_height.unwrap_or(f32::INFINITY)),
        );
        tree.set_style(root, root_style)?;
        tree.compute_layout_with_measure(root, available, |inputs, _, context, style| {
            taffy::compute_leaf_layout(
                inputs,
                style,
                |_, _| 0.0,
                |known, available| {
                    let intrinsic = match context.as_deref() {
                        Some(Kind::Label {
                            text: value,
                            font,
                            color,
                            ..
                        }) => measure(
                            font.as_ref().unwrap_or(&theme.font),
                            value,
                            color.unwrap_or(theme.text),
                        ),
                        Some(Kind::Button { text: value, .. }) => {
                            measure(&theme.font, value, theme.text)
                        }
                        Some(Kind::TextInput { initial, .. }) => {
                            let size = measure(
                                &theme.font,
                                if initial.is_empty() { "M" } else { initial },
                                theme.text,
                            );
                            Size::new(size.width.max(120.0), size.height)
                        }
                        Some(Kind::Image { data }) => data.size(),
                        _ => Size::default(),
                    };
                    let width = match available.width {
                        tf::AvailableSpace::Definite(limit) => intrinsic.width.min(limit.max(0.0)),
                        _ => intrinsic.width,
                    };
                    tf::Size {
                        width: known.width.unwrap_or(width),
                        height: known.height.unwrap_or(intrinsic.height),
                    }
                },
            )
        })?;
        let size = tree.layout(root)?.size;
        let origin = self
            .anchor
            .position(bounds, Size::new(size.width, size.height), self.margin);
        let mut prepared = PreparedView {
            bounds: Rect {
                origin,
                size: Size::new(size.width, size.height),
            },
            focused: if self.state.window_focused {
                self.state.focus.clone()
            } else {
                None
            },
            namespace: self.key.clone(),
            content_revision: self.content_revision,
            ..Default::default()
        };
        paint_tree(
            &tree,
            &nodes,
            &self.root,
            origin,
            bounds,
            &self.key,
            theme,
            &mut measure,
            &mut prepared,
            &mut self.state,
            &mut self.editing_text,
            &[],
        )?;
        prepared.button_visuals(
            theme,
            self.state
                .hover
                .as_ref()
                .map(|key| format!("{}/{}", self.key, key))
                .as_deref(),
            self.state
                .pressed
                .as_ref()
                .map(|key| format!("{}/{}", self.key, key))
                .as_deref(),
        );
        self.input_scene = Some(interaction::InputScene::from(&prepared));
        self.cached = Some(Cached {
            bounds,
            theme: theme.clone(),
            prepared,
        });
        Ok(&self.cached.as_ref().unwrap().prepared)
    }
}
fn build_tree(
    tree: &mut TaffyTree<Kind>,
    widget: &Widget,
    nodes: &mut HashMap<String, tf::NodeId>,
    keys: &mut HashSet<String>,
    state: &crate::input::State,
) -> Result<tf::NodeId, LayoutError> {
    if !keys.insert(widget.key.clone()) {
        return Err(LayoutError::DuplicateKey(widget.key.clone()));
    }
    let l = widget.layout;
    if ![
        l.padding.left,
        l.padding.right,
        l.padding.top,
        l.padding.bottom,
        l.gap,
        l.grow,
    ]
    .iter()
    .chain(
        [l.width, l.height, l.max_width, l.max_height]
            .iter()
            .flatten(),
    )
    .all(|v| v.is_finite() && *v >= 0.0)
    {
        return Err(LayoutError::InvalidGeometry);
    }
    let dimension = |v: Option<f32>| v.map(tf::length).unwrap_or(tf::auto());
    let style = tf::Style {
        flex_direction: if matches!(widget.kind, Kind::Row) {
            tf::FlexDirection::Row
        } else {
            tf::FlexDirection::Column
        },
        align_items: Some(match l.align {
            Align::Start => tf::AlignItems::START,
            Align::Center => tf::AlignItems::CENTER,
            Align::End => tf::AlignItems::END,
            Align::Stretch => tf::AlignItems::STRETCH,
        }),
        padding: tf::Rect {
            left: tf::length(l.padding.left),
            right: tf::length(l.padding.right),
            top: tf::length(l.padding.top),
            bottom: tf::length(l.padding.bottom),
        },
        gap: tf::Size {
            width: tf::length(l.gap),
            height: tf::length(l.gap),
        },
        size: tf::Size {
            width: dimension(l.width),
            height: dimension(l.height),
        },
        min_size: tf::Size {
            width: tf::length(0.0),
            height: tf::length(0.0),
        },
        max_size: tf::Size {
            width: l.max_width.map(tf::length).unwrap_or(tf::auto()),
            height: l.max_height.map(tf::length).unwrap_or(tf::auto()),
        },
        overflow: if matches!(widget.kind, Kind::Scroll) {
            taffy::geometry::Point {
                x: taffy::Overflow::Scroll,
                y: taffy::Overflow::Scroll,
            }
        } else {
            taffy::geometry::Point {
                x: taffy::Overflow::Visible,
                y: taffy::Overflow::Visible,
            }
        },
        flex_shrink: 1.0,
        flex_grow: l.grow,
        ..Default::default()
    };
    let children = widget
        .children
        .iter()
        .map(|child| build_tree(tree, child, nodes, keys, state))
        .collect::<Result<Vec<_>, _>>()?;
    if matches!(widget.kind, Kind::Scroll) {
        for child in &children {
            let mut child_style = tree.style(*child)?.clone();
            child_style.flex_shrink = 0.0;
            tree.set_style(*child, child_style)?;
        }
    }
    let node = if children.is_empty() {
        tree.new_leaf_with_context(
            style,
            match &widget.kind {
                Kind::TextInput {
                    placeholder,
                    disabled,
                    read_only,
                    ..
                } => Kind::TextInput {
                    initial: state
                        .editors
                        .get(&widget.key)
                        .map_or_else(String::new, |editor| editor.display().0),
                    placeholder: placeholder.clone(),
                    disabled: *disabled,
                    read_only: *read_only,
                },
                _ => widget.kind.clone(),
            },
        )?
    } else {
        tree.new_with_children(style, &children)?
    };
    nodes.insert(widget.key.clone(), node);
    Ok(node)
}
#[allow(clippy::too_many_arguments)]
fn paint_tree(
    tree: &TaffyTree<Kind>,
    nodes: &HashMap<String, tf::NodeId>,
    widget: &Widget,
    parent: Point,
    parent_clip: Rect,
    namespace: &str,
    theme: &Theme,
    measure: &mut impl FnMut(&Font, &str, Color) -> Size,
    out: &mut PreparedView,
    state: &mut crate::input::State,
    editing_text: &mut Option<TextSystem>,
    scroll_ancestors: &[String],
) -> Result<(), LayoutError> {
    let layout = tree.layout(nodes[&widget.key])?;
    let rect = Rect::new(
        parent.x + layout.location.x,
        parent.y + layout.location.y,
        layout.size.width,
        layout.size.height,
    );
    let clip = parent_clip.intersection(rect);
    // The root's layout location is normally zero; descendants accumulate local positions.
    out.rects.insert(widget.key.clone(), rect);
    let key = format!("{namespace}/{}", widget.key);
    let content = Rect::new(
        rect.origin.x + widget.layout.padding.left,
        rect.origin.y + widget.layout.padding.top,
        (rect.size.width - widget.layout.padding.left - widget.layout.padding.right).max(0.0),
        (rect.size.height - widget.layout.padding.top - widget.layout.padding.bottom).max(0.0),
    );
    let (role, value, disabled, read_only, selection) = match &widget.kind {
        Kind::Label { text, .. } => (SemanticRole::Label, Some(text.clone()), false, false, None),
        Kind::Button { text, disabled, .. } => (
            SemanticRole::Button,
            Some(text.clone()),
            *disabled,
            false,
            None,
        ),
        Kind::TextInput {
            disabled,
            read_only,
            ..
        } => {
            let editor = &state.editors[&widget.key];
            (
                SemanticRole::TextInput,
                Some(editor.value.clone()),
                *disabled,
                *read_only,
                Some((editor.anchor, editor.cursor)),
            )
        }
        Kind::Image { .. } => (SemanticRole::Image, None, false, false, None),
        Kind::Scroll => (SemanticRole::Scroll, None, false, false, None),
        _ => (SemanticRole::Group, None, false, false, None),
    };
    out.semantics.push(SemanticNode {
        key: widget.key.clone(),
        role,
        value,
        disabled,
        read_only,
        selection,
        label: widget.accessible_label.clone(),
        children: widget.children.iter().map(|c| c.key.clone()).collect(),
        rect,
        clip,
    });
    match &widget.kind {
        Kind::Card { style } => out.items.push(PaintItem::Card {
            key,
            rect,
            clip,
            style: style.unwrap_or(theme.card),
        }),
        Kind::Label {
            text,
            color,
            font,
            overflow,
            align,
        } => {
            let font = font.as_ref().unwrap_or(&theme.font);
            let (text, size) =
                crate::text::fit_with_measure(text, content.size.width, *overflow, |value| {
                    measure(font, value, color.unwrap_or(theme.text))
                });
            let x = match align {
                Align::Center => (content.size.width - size.width) / 2.0,
                Align::End => content.size.width - size.width,
                _ => 0.0,
            };
            out.items.push(PaintItem::Text {
                key,
                rect: Rect::new(
                    content.origin.x + x.max(0.0),
                    content.origin.y + (content.size.height - size.height).max(0.0) / 2.0,
                    size.width,
                    size.height,
                ),
                clip: content.intersection(clip),
                text,
                font: font.clone(),
                color: color.unwrap_or(theme.text),
            });
        }
        Kind::Button {
            text,
            action,
            disabled,
        } => {
            out.items.push(PaintItem::Card {
                key: key.clone(),
                rect,
                clip,
                style: theme.button,
            });
            let (text, size) = crate::text::fit_with_measure(
                text,
                content.size.width,
                TextOverflow::EllipsisEnd,
                |value| measure(&theme.font, value, theme.text),
            );
            out.items.push(PaintItem::Text {
                key: format!("{key}/label"),
                rect: Rect::new(
                    content.origin.x + (content.size.width - size.width).max(0.0) / 2.0,
                    content.origin.y + (content.size.height - size.height).max(0.0) / 2.0,
                    size.width,
                    size.height,
                ),
                clip: content.intersection(clip),
                text,
                font: theme.font.clone(),
                color: theme.text,
            });
            out.controls.push(ControlRegion {
                key: widget.key.clone(),
                rect,
                clip,
                content,
                kind: ControlKind::Button(action.clone()),
                disabled: *disabled,
                scroll_ancestors: scroll_ancestors.to_vec(),
                line: None,
                text_origin: content.origin,
                caret: Rect::default(),
            });
            out.hits.push(HitRegion {
                key,
                rect,
                clip,
                action: action.clone(),
                disabled: *disabled,
            });
        }
        Kind::Image { data } => {
            let scale = (content.size.width / data.width() as f32)
                .min(content.size.height / data.height() as f32);
            let size = Size::new(data.width() as f32 * scale, data.height() as f32 * scale);
            out.items.push(PaintItem::Image {
                key,
                rect: Rect::new(
                    content.origin.x + (content.size.width - size.width) / 2.0,
                    content.origin.y + (content.size.height - size.height) / 2.0,
                    size.width,
                    size.height,
                ),
                clip: content.intersection(clip),
                data: data.clone(),
            });
        }
        Kind::TextInput {
            placeholder,
            disabled,
            read_only,
            ..
        } => {
            let editor = state
                .editors
                .get_mut(&widget.key)
                .expect("input state initialized");
            let (display, cursor) = editor.display();
            let showing_placeholder = display.is_empty();
            let value = if showing_placeholder {
                placeholder.as_str()
            } else {
                display.as_str()
            };
            let service = editing_text.get_or_insert_with(TextSystem::new);
            let line = service.line_geometry(&theme.font, &display);
            let caret_x = line.x(cursor);
            let size = measure(&theme.font, value, theme.text);
            let focused =
                state.window_focused && state.focus.as_deref() == Some(widget.key.as_str());
            if focused {
                editor.offset = editor
                    .offset
                    .max(caret_x - content.size.width + 2.0)
                    .min(caret_x)
                    .max(0.0);
            }
            editor.offset = editor
                .offset
                .min((line.width - content.size.width + 2.0).max(0.0));
            let origin = Point::new(
                content.origin.x - editor.offset,
                content.origin.y + (content.size.height - line.height).max(0.0) / 2.0,
            );
            let mut style = theme.button;
            style.border_width = if focused { 2.0 } else { 1.0 };
            if *disabled {
                style.fill.a *= 0.5;
            }
            out.items.push(PaintItem::Card {
                key: key.clone(),
                rect,
                clip,
                style,
            });
            if focused && editor.preedit.is_none() {
                let selection = editor.selection();
                for (i, (_, _, x, w)) in
                    line.clusters
                        .iter()
                        .enumerate()
                        .filter(|(_, (start, end, _, _))| {
                            *end > selection.start && *start < selection.end
                        })
                {
                    out.items.push(PaintItem::Card {
                        key: format!("{key}/selection/{i}"),
                        rect: Rect::new(origin.x + x, origin.y, *w, line.height),
                        clip: content.intersection(clip),
                        style: CardStyle {
                            fill: Color::rgba(0.3, 0.5, 0.8, 0.5),
                            border: theme.text,
                            border_width: 0.0,
                            radius: 0.0,
                        },
                    });
                }
            }
            out.items.push(PaintItem::Text {
                key: format!("{key}/text"),
                rect: Rect::new(origin.x, origin.y, size.width, size.height),
                clip: content.intersection(clip),
                text: value.into(),
                font: theme.font.clone(),
                color: if showing_placeholder {
                    theme.subtext
                } else {
                    theme.text
                },
            });
            let caret = Rect::new(origin.x + caret_x, origin.y, 1.0, line.height);
            if focused {
                out.items.push(PaintItem::Card {
                    key: format!("{key}/caret"),
                    rect: caret,
                    clip: content.intersection(clip),
                    style: CardStyle {
                        fill: theme.text,
                        border: theme.text,
                        border_width: 0.0,
                        radius: 0.0,
                    },
                });
                if let Some((preedit, _)) = &editor.preedit {
                    let start = editor.selection().start;
                    let left = line.x(start);
                    let right = line.x(start + preedit.len());
                    out.items.push(PaintItem::Card {
                        key: format!("{key}/preedit"),
                        rect: Rect::new(
                            origin.x + left.min(right),
                            origin.y + line.height - 1.0,
                            (right - left).abs(),
                            1.0,
                        ),
                        clip: content.intersection(clip),
                        style: CardStyle {
                            fill: theme.text,
                            border: theme.text,
                            border_width: 0.0,
                            radius: 0.0,
                        },
                    });
                }
            }
            out.controls.push(ControlRegion {
                key: widget.key.clone(),
                rect,
                clip,
                content,
                kind: ControlKind::TextInput {
                    read_only: *read_only,
                },
                disabled: *disabled,
                scroll_ancestors: scroll_ancestors.to_vec(),
                line: Some(line),
                text_origin: origin,
                caret,
            });
        }
        Kind::Row | Kind::Column | Kind::Scroll => {}
    }
    if state.window_focused
        && state.focus.as_deref() == Some(widget.key.as_str())
        && matches!(widget.kind, Kind::Button { .. })
    {
        out.items.push(PaintItem::Card {
            key: format!("{namespace}/{}/focus", widget.key),
            rect,
            clip,
            style: CardStyle {
                fill: Color::rgba(0.0, 0.0, 0.0, 0.0),
                border: theme.card.border,
                border_width: 2.0,
                radius: theme.button.radius,
            },
        });
    }
    let mut child_origin = rect.origin;
    let mut ancestors = scroll_ancestors.to_vec();
    if matches!(widget.kind, Kind::Scroll) {
        let mut extent = content.size;
        for child in &widget.children {
            let child_layout = tree.layout(nodes[&child.key])?;
            extent.width = extent.width.max(
                child_layout.location.x + child_layout.size.width - widget.layout.padding.left,
            );
            extent.height = extent.height.max(
                child_layout.location.y + child_layout.size.height - widget.layout.padding.top,
            );
        }
        let offset = state.scroll.entry(widget.key.clone()).or_default();
        offset.x = offset
            .x
            .clamp(0.0, (extent.width - content.size.width).max(0.0));
        offset.y = offset
            .y
            .clamp(0.0, (extent.height - content.size.height).max(0.0));
        out.scrolls.push(ScrollRegion {
            key: widget.key.clone(),
            viewport: content,
            clip: content.intersection(clip),
            extent,
            offset: *offset,
            ancestors: ancestors.clone(),
        });
        child_origin.x -= offset.x;
        child_origin.y -= offset.y;
        ancestors.push(widget.key.clone());
    }
    for child in &widget.children {
        paint_tree(
            tree,
            nodes,
            child,
            child_origin,
            content.intersection(clip),
            namespace,
            theme,
            measure,
            out,
            state,
            editing_text,
            &ancestors,
        )?;
    }
    Ok(())
}
