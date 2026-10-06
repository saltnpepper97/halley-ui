//! Shared Cosmic Text shaping, font resolution, Swash glyph rasterization, and caching.
//! Extracted from Halley's compositor renderer; pixels are premultiplied RGBA.
use crate::Size;
use cosmic_text::{
    Attrs, Buffer as TextBuffer, Color, Family, FontSystem, Hinting, Metrics, Shaping, Style,
    SwashCache, Weight,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation;
const HINTING_MAX_SIZE_PX: u16 = 16;
const RASTER_TRAILING_PAD_PX: i32 = 2;
const MAX_CACHE_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 256;
const CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Font {
    pub family: String,
    pub size: u16,
}
impl Default for Font {
    fn default() -> Self {
        Self {
            family: "monospace".into(),
            size: 11,
        }
    }
}
impl Font {
    pub fn normalized(&self) -> Self {
        Self {
            family: if self.family.trim().is_empty() {
                "monospace".into()
            } else {
                self.family.trim().into()
            },
            size: self.size.max(1),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextOverflow {
    #[default]
    Clip,
    EllipsisEnd,
    EllipsisMiddle,
}
#[derive(Clone, Debug)]
pub struct TextRaster {
    pub pixels: Arc<[u8]>,
    pub width: i32,
    pub height: i32,
}
impl TextRaster {
    pub fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Key {
    font: Font,
    text: String,
    rgb: [u8; 3],
}
struct Entry {
    raster: Arc<TextRaster>,
    used: Instant,
}
/// One instance per rendering thread. Font and glyph caches are shared by all its consumers.
pub struct TextSystem {
    fonts: FontSystem,
    swash: SwashCache,
    cache: HashMap<Key, Entry>,
    lines: HashMap<(Font, String), Arc<LineGeometry>>,
}
impl Default for TextSystem {
    fn default() -> Self {
        Self::new()
    }
}
impl TextSystem {
    pub fn new() -> Self {
        Self {
            fonts: FontSystem::new(),
            swash: SwashCache::new(),
            cache: HashMap::new(),
            lines: HashMap::new(),
        }
    }
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lines.clear();
    }
    /// GPU consumers already cache textures; avoid retaining a second CPU pixel copy.
    pub fn raster_uncached(
        &mut self,
        font: &Font,
        text: &str,
        rgb: [u8; 3],
    ) -> Option<Arc<TextRaster>> {
        if text.is_empty() {
            return None;
        }
        let font = font.normalized();
        let (pixels, (width, height)) = raster_text(
            &mut self.fonts,
            &mut self.swash,
            text,
            font.size,
            rgb,
            &font.family,
        )?;
        Some(Arc::new(TextRaster {
            pixels: pixels.into(),
            width,
            height,
        }))
    }
    pub fn raster(&mut self, font: &Font, text: &str, rgb: [u8; 3]) -> Option<Arc<TextRaster>> {
        if text.is_empty() {
            return None;
        }
        let key = Key {
            font: font.normalized(),
            text: text.into(),
            rgb,
        };
        let now = Instant::now();
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.used = now;
            return Some(entry.raster.clone());
        }
        let (pixels, (width, height)) = raster_text(
            &mut self.fonts,
            &mut self.swash,
            text,
            key.font.size,
            rgb,
            &key.font.family,
        )?;
        let raster = Arc::new(TextRaster {
            pixels: pixels.into(),
            width,
            height,
        });
        self.cache
            .retain(|_, entry| now.saturating_duration_since(entry.used) < CACHE_TTL);
        let mut bytes: usize = self.cache.values().map(|e| e.raster.pixels.len()).sum();
        while !self.cache.is_empty()
            && (self.cache.len() >= MAX_CACHE_ENTRIES
                || bytes + raster.pixels.len() > MAX_CACHE_BYTES)
        {
            let oldest = self
                .cache
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| key.clone())
                .unwrap();
            bytes -= self.cache.remove(&oldest).unwrap().raster.pixels.len();
        }
        if raster.pixels.len() <= MAX_CACHE_BYTES {
            self.cache.insert(
                key,
                Entry {
                    raster: raster.clone(),
                    used: now,
                },
            );
        }
        Some(raster)
    }
    pub fn measure(&mut self, font: &Font, text: &str) -> Size {
        self.raster(font, text, [255; 3])
            .map(|r| r.size())
            .unwrap_or_default()
    }
    /// Shaped grapheme positions for cursor hit testing, selection, and IME geometry.
    /// Uses the same font resolution and hinting as glyph rasterization; no pixel copy.
    pub fn line_geometry(&mut self, font: &Font, text: &str) -> Arc<LineGeometry> {
        let font = font.normalized();
        let key = (font.clone(), text.to_owned());
        if let Some(line) = self.lines.get(&key) {
            return line.clone();
        }
        let mut buffer = shape_buffer(&mut self.fonts, text, font.size, &font.family);
        buffer.shape_until_scroll(&mut self.fonts, true);
        let mut line = LineGeometry::default();
        if let Some(run) = buffer.layout_runs().next() {
            line.height = run.line_height;
            line.width = run.line_w;
            for index in text
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(text.len()))
            {
                let x = run
                    .cursor_position(&cosmic_text::Cursor::new(0, index))
                    .unwrap_or(0.0);
                line.stops.push((index, x));
            }
            for glyph in run.glyphs {
                let clusters: Vec<_> = text[glyph.start..glyph.end]
                    .grapheme_indices(true)
                    .collect();
                let step = glyph.w / clusters.len().max(1) as f32;
                for (i, (start, value)) in clusters.iter().enumerate() {
                    let x = if glyph.level.is_rtl() {
                        glyph.x + glyph.w - (i + 1) as f32 * step
                    } else {
                        glyph.x + i as f32 * step
                    };
                    line.clusters.push((
                        glyph.start + start,
                        glyph.start + start + value.len(),
                        x,
                        step,
                    ));
                }
            }
        }
        line.width = line
            .width
            .max(line.stops.iter().map(|(_, x)| *x).fold(0.0, f32::max));
        if line.stops.is_empty() {
            line.stops.push((0, 0.0));
            line.height = (font.size as f32 * 1.25).ceil();
        }
        let line = Arc::new(line);
        if self.lines.len() >= MAX_CACHE_ENTRIES {
            self.lines.clear();
        }
        self.lines.insert(key, line.clone());
        line
    }
    /// Grapheme-safe truncation. If even an ellipsis does not fit, returns empty text.
    pub fn fit(
        &mut self,
        font: &Font,
        text: &str,
        width: f32,
        overflow: TextOverflow,
    ) -> (String, Size) {
        fit_with_measure(text, width, overflow, |value| self.measure(font, value))
    }
}
pub fn fit_with_measure(
    text: &str,
    width: f32,
    overflow: TextOverflow,
    mut measure: impl FnMut(&str) -> Size,
) -> (String, Size) {
    let measured = measure(text);
    if measured.width <= width || overflow == TextOverflow::Clip {
        return (text.into(), measured);
    }
    let ellipsis = measure("…");
    if ellipsis.width > width {
        return (String::new(), Size::new(0.0, measured.height));
    }
    let clusters: Vec<_> = text.graphemes(true).collect();
    for keep in (0..clusters.len()).rev() {
        let value = match overflow {
            TextOverflow::EllipsisEnd => format!("{}…", clusters[..keep].concat()),
            TextOverflow::EllipsisMiddle => {
                let left = keep.div_ceil(2);
                let right = keep / 2;
                format!(
                    "{}…{}",
                    clusters[..left].concat(),
                    clusters[clusters.len() - right..].concat()
                )
            }
            TextOverflow::Clip => unreachable!(),
        };
        let size = measure(&value);
        if size.width <= width {
            return (value, size);
        }
    }
    (String::new(), Size::new(0.0, measured.height))
}
/// Single-line shaped geometry. All indices are UTF-8 grapheme boundaries.
#[derive(Clone, Debug, Default)]
pub struct LineGeometry {
    pub width: f32,
    pub height: f32,
    pub stops: Vec<(usize, f32)>,
    pub clusters: Vec<(usize, usize, f32, f32)>,
}
impl LineGeometry {
    pub fn x(&self, index: usize) -> f32 {
        self.stops
            .iter()
            .find(|(i, _)| *i == index)
            .map_or(0.0, |(_, x)| *x)
    }
    pub fn hit(&self, x: f32) -> usize {
        self.stops
            .iter()
            .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
            .map_or(0, |(i, _)| *i)
    }
}
fn shape_buffer(
    font_system: &mut FontSystem,
    text: &str,
    size_px: u16,
    family: &str,
) -> TextBuffer {
    let font_size = size_px.max(1) as f32;
    let mut buffer = TextBuffer::new(
        font_system,
        Metrics::new(font_size, (font_size * 1.25).ceil()),
    );
    buffer.set_hinting(if size_px <= HINTING_MAX_SIZE_PX {
        Hinting::Enabled
    } else {
        Hinting::Disabled
    });
    buffer.set_size(None, None);
    let request = parse_font_request(family);
    let resolved = resolve_named_family(font_system, request.family);
    buffer.set_text(
        text,
        &Attrs::new()
            .family(resolve_family(
                resolved.as_deref().unwrap_or(request.family),
            ))
            .style(request.style)
            .weight(request.weight),
        Shaping::Advanced,
        None,
    );
    buffer
}
pub(crate) fn raster_text(
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text: &str,
    size_px: u16,
    rgb: [u8; 3],
    family: &str,
) -> Option<(Vec<u8>, (i32, i32))> {
    let font_size = size_px.max(1) as f32;
    let mut buffer = shape_buffer(font_system, text, size_px, family);
    buffer.shape_until_scroll(font_system, true);

    let mut width = 0_i32;
    let mut height = 0_i32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w.ceil() as i32);
        height = height.max((run.line_top + run.line_height).ceil() as i32);
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, run.line_y), 1.0);
            if let Some(image) = swash_cache
                .get_image(font_system, physical.cache_key)
                .as_ref()
            {
                let glyph_width = i32::try_from(image.placement.width).unwrap_or(i32::MAX);
                let right = physical
                    .x
                    .saturating_add(image.placement.left)
                    .saturating_add(glyph_width);
                width = width.max(right);
            }
        }
    }
    width = width.max(1).saturating_add(RASTER_TRAILING_PAD_PX);
    height = height.max((font_size * 1.25).ceil() as i32).max(1);

    let mut pixels = vec![0_u8; width as usize * height as usize * 4];
    let color = Color::rgba(rgb[0], rgb[1], rgb[2], 255);
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, run.line_y), 1.0);
            swash_cache.with_pixels(font_system, physical.cache_key, color, |gx, gy, pixel| {
                let x = physical.x + gx;
                let y = physical.y + gy;
                if x < 0 || y < 0 || x >= width || y >= height {
                    return;
                }
                let index = ((y as usize * width as usize) + x as usize) * 4;
                let alpha = pixel.a() as f32 / 255.0;
                // Smithay's GLES path expects premultiplied RGBA.
                pixels[index] = (pixel.r() as f32 * alpha).round() as u8;
                pixels[index + 1] = (pixel.g() as f32 * alpha).round() as u8;
                pixels[index + 2] = (pixel.b() as f32 * alpha).round() as u8;
                pixels[index + 3] = pixel.a();
            });
        }
    }
    Some((pixels, (width, height)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FontRequest<'a> {
    family: &'a str,
    style: Style,
    weight: Weight,
}

fn parse_font_request(requested: &str) -> FontRequest<'_> {
    let trimmed = requested.trim();
    let mut family = if trimmed.is_empty() {
        "monospace"
    } else {
        trimmed
    };
    let mut style = Style::Normal;
    let mut weight = Weight::NORMAL;
    loop {
        if style == Style::Normal {
            if let Some(stripped) = strip_suffix(family, &[" italic"]) {
                family = stripped;
                style = Style::Italic;
                continue;
            }
            if let Some(stripped) = strip_suffix(family, &[" oblique"]) {
                family = stripped;
                style = Style::Oblique;
                continue;
            }
        }
        if weight == Weight::NORMAL {
            for (suffixes, parsed) in [
                (
                    &[" extra bold", " extra-bold", " extrabold"][..],
                    Weight::EXTRA_BOLD,
                ),
                (
                    &[" semi bold", " semi-bold", " semibold"][..],
                    Weight::SEMIBOLD,
                ),
                (
                    &[" extra light", " extra-light", " extralight"][..],
                    Weight::EXTRA_LIGHT,
                ),
                (&[" bold"][..], Weight::BOLD),
                (&[" medium"][..], Weight::MEDIUM),
                (&[" light"][..], Weight::LIGHT),
                (&[" thin"][..], Weight::THIN),
                (&[" black"][..], Weight::BLACK),
            ] {
                if let Some(stripped) = strip_suffix(family, suffixes) {
                    family = stripped;
                    weight = parsed;
                    break;
                }
            }
            if weight != Weight::NORMAL {
                continue;
            }
        }
        break;
    }
    FontRequest {
        family,
        style,
        weight,
    }
}

fn strip_suffix<'a>(family: &'a str, suffixes: &[&str]) -> Option<&'a str> {
    let folded = family.to_ascii_lowercase();
    suffixes.iter().find_map(|suffix| {
        folded
            .ends_with(suffix)
            .then(|| family[..family.len() - suffix.len()].trim_end())
    })
}

fn resolve_family(family: &str) -> Family<'_> {
    match family.trim().to_ascii_lowercase().as_str() {
        "serif" => Family::Serif,
        "sans-serif" | "sans_serif" | "sansserif" | "sans" => Family::SansSerif,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        "monospace" => Family::Monospace,
        _ => Family::Name(family),
    }
}

fn resolve_named_family(font_system: &FontSystem, requested: &str) -> Option<String> {
    if matches!(
        resolve_family(requested),
        Family::Serif | Family::SansSerif | Family::Cursive | Family::Fantasy | Family::Monospace
    ) {
        return None;
    }
    let requested = fold_font_name(requested);
    font_system.db().faces().find_map(|face| {
        face.families
            .iter()
            .map(|(family, _)| family.as_str())
            .find(|family| fold_font_name(family) == requested)
            .map(str::to_string)
            .or_else(|| {
                (fold_font_name(&face.post_script_name) == requested)
                    .then(|| face.families.first().map(|(family, _)| family.clone()))
                    .flatten()
            })
    })
}

fn fold_font_name(name: &str) -> String {
    name.chars()
        .filter(|character| !matches!(character, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_suffixes_select_weight_and_style_without_losing_the_name() {
        let parsed = parse_font_request("CommitMono Nerd Font Semi-Bold Italic");
        assert_eq!(parsed.family, "CommitMono Nerd Font");
        assert_eq!(parsed.weight, Weight::SEMIBOLD);
        assert_eq!(parsed.style, Style::Italic);
    }
}
