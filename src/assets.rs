//! Prepared image/icon assets. File reads and decoding belong on a worker, not in paint.
use crate::Size;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct ImageData {
    pixels: Arc<[u8]>,
    width: u32,
    height: u32,
    id: u64,
}
static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("image dimensions or buffer length are invalid")]
    InvalidBuffer,
    #[error("image I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("image decode: {0}")]
    Decode(#[from] image::ImageError),
    #[error("SVG decode: {0}")]
    Svg(#[from] resvg::usvg::Error),
}
impl ImageData {
    /// Premultiplied RGBA bytes, shared without copying between layouts and renderers.
    pub fn from_rgba(width: u32, height: u32, pixels: Arc<[u8]>) -> Result<Self, ImageError> {
        let bytes = (width as usize)
            .checked_mul(height as usize)
            .and_then(|v| v.checked_mul(4));
        if width == 0 || height == 0 || bytes != Some(pixels.len()) {
            return Err(ImageError::InvalidBuffer);
        }
        Ok(Self {
            width,
            height,
            pixels,
            id: NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
        })
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    /// Content generation used by retained GPU render elements; never derived from an address.
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }
    /// Decode a raster image or SVG into a bounded asset; caller selects the pixel budget.
    pub fn load(path: &Path, max_dimension: u32) -> Result<Self, ImageError> {
        let max_dimension = max_dimension.clamp(1, 4096);
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
        {
            let data = std::fs::read(path)?;
            return Self::from_svg(&data, max_dimension);
        }
        let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader
            .decode()?
            .thumbnail(max_dimension, max_dimension)
            .into_rgba8();
        let (width, height) = decoded.dimensions();
        let mut pixels = decoded.into_raw();
        for pixel in pixels.chunks_exact_mut(4) {
            for channel in 0..3 {
                pixel[channel] =
                    ((u16::from(pixel[channel]) * u16::from(pixel[3]) + 127) / 255) as u8;
            }
        }
        Self::from_rgba(width, height, pixels.into())
    }
    pub fn from_svg(data: &[u8], max_dimension: u32) -> Result<Self, ImageError> {
        let tree = resvg::usvg::Tree::from_data(data, &resvg::usvg::Options::default())?;
        let original = tree.size();
        let limit = max_dimension.clamp(1, 4096) as f32;
        let scale = (limit / original.width().max(original.height())).min(1.0);
        let width = (original.width() * scale).ceil().max(1.0) as u32;
        let height = (original.height() * scale).ceil().max(1.0) as u32;
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or(ImageError::InvalidBuffer)?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        Self::from_rgba(width, height, pixmap.take().into())
    }
}
