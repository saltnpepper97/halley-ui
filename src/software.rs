#![doc = include_str!("../docs/rendering.md")]
//! Software drawing into owned or borrowed premultiplied pixel buffers.
//! A Wayland client can borrow a SHM canvas directly; no intermediate full-frame copy.
use crate::{
    Color, Point, Rect, Size,
    assets::ImageData,
    text::TextSystem,
    ui::{PaintItem, PreparedView},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba,
    Bgra,
}
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("invalid pixel dimensions, stride, or buffer length")]
    InvalidDimensions,
    #[error("pixel buffer allocation failed")]
    Allocation,
}
pub struct PixelBuffer {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}
impl PixelBuffer {
    pub fn new(width: u32, height: u32) -> Result<Self, BufferError> {
        let bytes = buffer_len(width, height, width as usize * 4)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(bytes)
            .map_err(|_| BufferError::Allocation)?;
        pixels.resize(bytes, 0);
        Ok(Self {
            pixels,
            width,
            height,
        })
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn surface(&mut self) -> Surface<'_> {
        Surface {
            pixels: &mut self.pixels,
            width: self.width,
            height: self.height,
            stride: self.width as usize * 4,
            format: PixelFormat::Rgba,
        }
    }
}
pub struct Surface<'a> {
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
    stride: usize,
    format: PixelFormat,
}
fn buffer_len(width: u32, height: u32, stride: usize) -> Result<usize, BufferError> {
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || stride < width as usize * 4
    {
        return Err(BufferError::InvalidDimensions);
    }
    stride
        .checked_mul(height as usize)
        .ok_or(BufferError::InvalidDimensions)
}
impl<'a> Surface<'a> {
    pub fn new(
        pixels: &'a mut [u8],
        width: u32,
        height: u32,
        stride: usize,
        format: PixelFormat,
    ) -> Result<Self, BufferError> {
        if pixels.len() < buffer_len(width, height, stride)? {
            return Err(BufferError::InvalidDimensions);
        }
        Ok(Self {
            pixels,
            width,
            height,
            stride,
            format,
        })
    }
    pub fn clear(&mut self, color: Color) {
        let bytes = color.premultiplied().map(|v| (v * 255.0).round() as u8);
        let bytes = if self.format == PixelFormat::Bgra {
            [bytes[2], bytes[1], bytes[0], bytes[3]]
        } else {
            bytes
        };
        let row_bytes = self.width as usize * 4;
        for row in self.pixels[..self.stride * self.height as usize].chunks_exact_mut(self.stride) {
            let row = &mut row[..row_bytes];
            if bytes == [0; 4] {
                row.fill(0);
            } else {
                for pixel in row.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&bytes);
                }
            }
        }
    }
    pub fn draw(&mut self, view: &PreparedView, text: &mut TextSystem, opacity: f32) {
        let opacity = opacity.clamp(0.0, 1.0);
        if opacity == 0.0 {
            return;
        }
        for item in &view.items {
            match item {
                PaintItem::Card {
                    rect, clip, style, ..
                } => {
                    // Invisible container chrome must not rasterize its entire bounds.
                    if style.fill.a <= 0.0 && style.border.a <= 0.0 {
                        continue;
                    }
                    let border = style
                        .border_width
                        .max(0.0)
                        .min(rect.size.width.min(rect.size.height) * 0.5);
                    let content_radius = style
                        .radius
                        .max(0.0)
                        .min((rect.size.width.min(rect.size.height) * 0.5 - border).max(0.0));
                    let radius = if content_radius > 0.0 {
                        content_radius + border
                    } else {
                        0.0
                    };
                    let inner_offset = border + 0.75;
                    let inner_radius = (content_radius - 0.75).max(0.0);
                    let inner = Rect::new(
                        rect.origin.x + inner_offset,
                        rect.origin.y + inner_offset,
                        (rect.size.width - inner_offset * 2.0).max(1.0),
                        (rect.size.height - inner_offset * 2.0).max(1.0),
                    );
                    let fill = style.fill.premultiplied();
                    let stroke = style.border.premultiplied();
                    let solid = fill.map(|channel| channel * opacity);
                    // Interior spans normally sit on one constant background. Reuse
                    // the exact source-over result instead of repeating float blending.
                    let mut solid_cache: Option<([u8; 4], [u8; 4])> = None;
                    self.for_rect(rect.intersection(*clip), |surface, x, y| {
                        let p = Point::new(x as f32 + 0.5, y as f32 + 0.5);
                        let outer = coverage(sdf(p, *rect, radius));
                        let inside = if border > 0.0 {
                            coverage(sdf(p, inner, inner_radius))
                        } else {
                            outer
                        };
                        let edge = (outer - inside).max(0.0);
                        if inside == 1.0 && edge == 0.0 {
                            let index = y as usize * surface.stride + x as usize * 4;
                            let old: [u8; 4] = surface.pixels[index..index + 4].try_into().unwrap();
                            if let Some((previous, output)) = solid_cache
                                && previous == old
                            {
                                surface.pixels[index..index + 4].copy_from_slice(&output);
                            } else {
                                surface.blend(x, y, solid);
                                solid_cache = Some((
                                    old,
                                    surface.pixels[index..index + 4].try_into().unwrap(),
                                ));
                            }
                            return;
                        }
                        surface.blend(
                            x,
                            y,
                            std::array::from_fn(|i| {
                                (fill[i] * inside + stroke[i] * edge) * opacity
                            }),
                        );
                    });
                }
                PaintItem::Text {
                    rect,
                    clip,
                    text: value,
                    font,
                    color,
                    ..
                } => {
                    if let Some(raster) = text.raster(font, value, color.bytes()) {
                        self.blit(
                            &raster.pixels,
                            raster.width as u32,
                            raster.height as u32,
                            *rect,
                            *clip,
                            opacity * color.a.clamp(0.0, 1.0),
                        );
                    }
                }
                PaintItem::Image {
                    rect, clip, data, ..
                } => self.blit(
                    data.pixels(),
                    data.width(),
                    data.height(),
                    *rect,
                    *clip,
                    opacity,
                ),
            }
        }
    }
    pub fn image(&mut self, image: &ImageData, rect: Rect, opacity: f32) {
        self.blit(
            image.pixels(),
            image.width(),
            image.height(),
            rect,
            rect,
            opacity.clamp(0.0, 1.0),
        );
    }
    fn blit(
        &mut self,
        pixels: &[u8],
        width: u32,
        height: u32,
        destination: Rect,
        clip: Rect,
        opacity: f32,
    ) {
        if destination.size.width <= 0.0 || destination.size.height <= 0.0 {
            return;
        }
        self.for_rect(destination.intersection(clip), |surface, x, y| {
            let sx = (((x as f32 + 0.5 - destination.origin.x) / destination.size.width)
                * width as f32)
                .floor()
                .clamp(0.0, width as f32 - 1.0) as usize;
            let sy = (((y as f32 + 0.5 - destination.origin.y) / destination.size.height)
                * height as f32)
                .floor()
                .clamp(0.0, height as f32 - 1.0) as usize;
            let start = (sy * width as usize + sx) * 4;
            if pixels[start..start + 4] == [0; 4] {
                return;
            }
            surface.blend(
                x,
                y,
                std::array::from_fn(|i| pixels[start + i] as f32 / 255.0 * opacity),
            );
        });
    }
    fn for_rect(&mut self, rect: Rect, mut draw: impl FnMut(&mut Self, u32, u32)) {
        let bounds = rect.intersection(Rect {
            origin: Point::default(),
            size: Size::new(self.width as f32, self.height as f32),
        });
        for y in bounds.origin.y.floor().max(0.0) as u32
            ..(bounds.origin.y + bounds.size.height)
                .ceil()
                .min(self.height as f32) as u32
        {
            for x in bounds.origin.x.floor().max(0.0) as u32
                ..(bounds.origin.x + bounds.size.width)
                    .ceil()
                    .min(self.width as f32) as u32
            {
                draw(self, x, y);
            }
        }
    }
    fn write(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let index = y as usize * self.stride + x as usize * 4;
        let ordered = if self.format == PixelFormat::Bgra {
            [rgba[2], rgba[1], rgba[0], rgba[3]]
        } else {
            rgba
        };
        self.pixels[index..index + 4].copy_from_slice(&ordered);
    }
    fn blend(&mut self, x: u32, y: u32, rgba: [f32; 4]) {
        let index = y as usize * self.stride + x as usize * 4;
        let old = &self.pixels[index..index + 4];
        let old = if self.format == PixelFormat::Bgra {
            [old[2], old[1], old[0], old[3]]
        } else {
            [old[0], old[1], old[2], old[3]]
        };
        let a = rgba[3].clamp(0.0, 1.0);
        let combined = std::array::from_fn(|i| {
            ((rgba[i] + old[i] as f32 / 255.0 * (1.0 - a)).clamp(0.0, 1.0) * 255.0).round() as u8
        });
        self.write(x, y, combined);
    }
}
fn sdf(point: Point, rect: Rect, radius: f32) -> f32 {
    let half = Size::new(rect.size.width * 0.5, rect.size.height * 0.5);
    let radius = radius.min(half.width.min(half.height)).max(0.0);
    let qx = (point.x - rect.origin.x - half.width).abs() - half.width + radius;
    let qy = (point.y - rect.origin.y - half.height).abs() - half.height + radius;
    // Only rounded corners need a distance calculation. Interior and straight
    // edges have an exact one-dimensional distance, including the AA fringe.
    let outside = if qx > 0.0 && qy > 0.0 {
        qx.hypot(qy)
    } else {
        qx.max(qy).max(0.0)
    };
    outside + qx.max(qy).min(0.0) - radius
}
fn coverage(distance: f32) -> f32 {
    let t = ((distance + 0.75) / 1.5).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}
