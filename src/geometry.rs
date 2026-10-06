//! Output-local geometry in UI units; hosts convert to their output convention.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}
impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}
impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.origin.x
            && p.y >= self.origin.y
            && p.x < self.origin.x + self.size.width
            && p.y < self.origin.y + self.size.height
    }
    pub fn translated(self, offset: Point) -> Self {
        Self {
            origin: Point::new(self.origin.x + offset.x, self.origin.y + offset.y),
            ..self
        }
    }
    pub fn intersection(self, other: Self) -> Self {
        let x = self.origin.x.max(other.origin.x);
        let y = self.origin.y.max(other.origin.y);
        Self::new(
            x,
            y,
            (self.origin.x + self.size.width)
                .min(other.origin.x + other.size.width)
                .max(x)
                - x,
            (self.origin.y + self.size.height)
                .min(other.origin.y + other.size.height)
                .max(y)
                - y,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}
impl Insets {
    pub const fn xy(x: f32, y: f32) -> Self {
        Self {
            left: x,
            right: x,
            top: y,
            bottom: y,
        }
    }
    pub const fn all(value: f32) -> Self {
        Self::xy(value, value)
    }
}

/// Straight-alpha sRGB color. Renderers premultiply exactly once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }
    pub fn bytes(self) -> [u8; 3] {
        [self.r, self.g, self.b].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
    }
    pub fn premultiplied(self) -> [f32; 4] {
        let a = self.a.clamp(0.0, 1.0);
        [
            self.r.clamp(0.0, 1.0) * a,
            self.g.clamp(0.0, 1.0) * a,
            self.b.clamp(0.0, 1.0) * a,
            a,
        ]
    }
}
