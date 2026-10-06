//! Shared card shaders and Smithay elements extracted from Halley.
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::texture::TextureRenderElement;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
    UniformType,
};
use smithay::backend::renderer::utils::{CommitCounter, OpaqueRegions};
use smithay::backend::renderer::{ContextId, ImportMem, Renderer, Texture};
#[cfg(feature = "smithay-render-cache")]
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Logical, Physical, Rectangle, Scale, Size, Transform};
use std::{
    error::Error,
    hash::{DefaultHasher, Hash, Hasher},
};
const LABEL_SQUARE: &str = include_str!("shaders/ui_rect_square.frag");
const LABEL_ROUNDED: &str = include_str!("shaders/ui_rect_rounded.frag");
#[derive(Clone, Copy, Debug)]
pub struct CardMetrics {
    pub content_radius: f32,
    pub border_width: f32,
    pub outer_radius: f32,
    pub inner_offset: f32,
    pub inner_radius: f32,
}
pub fn card_metrics(size: Size<i32, Physical>, radius: f32, border: f32) -> CardMetrics {
    let border = border.max(0.0).min(size.w.min(size.h).max(0) as f32 * 0.5);
    let radius = radius
        .max(0.0)
        .min((size.w.min(size.h).max(0) as f32 * 0.5 - border).max(0.0));
    CardMetrics {
        content_radius: radius,
        border_width: border,
        outer_radius: if radius > 0.0 { radius + border } else { 0.0 },
        inner_offset: border + 0.75,
        inner_radius: (radius - 0.75).max(0.0),
    }
}
fn finish_commit(hasher: DefaultHasher) -> CommitCounter {
    CommitCounter::from(hasher.finish() as usize)
}
fn hash_floats(values: impl IntoIterator<Item = f32>, hasher: &mut DefaultHasher) {
    for value in values {
        value.to_bits().hash(hasher);
    }
}
#[allow(clippy::too_many_arguments)]
fn label_commit(
    rounded: bool,
    fill: (f32, f32, f32, f32),
    border: (f32, f32, f32, f32),
    destination: Rectangle<i32, Physical>,
    corner_radius: f32,
    inner_offset: f32,
    inner_corner_radius: f32,
    border_px: f32,
) -> CommitCounter {
    let mut hasher = DefaultHasher::new();
    rounded.hash(&mut hasher);
    destination.size.w.hash(&mut hasher);
    destination.size.h.hash(&mut hasher);
    hash_floats(
        [
            fill.0,
            fill.1,
            fill.2,
            fill.3,
            border.0,
            border.1,
            border.2,
            border.3,
            corner_radius,
            inner_offset,
            inner_corner_radius,
            border_px,
        ],
        &mut hasher,
    );
    finish_commit(hasher)
}

#[derive(Clone, Copy, Debug)]
pub struct OverlayCardStyle {
    pub content_radius: f32,
    pub fill: (f32, f32, f32, f32),
    pub border: (f32, f32, f32, f32),
    pub border_px: f32,
    pub alpha: f32,
}

#[derive(Debug)]
pub struct LabelRenderElement {
    base: TextureRenderElement<GlesTexture>,
    texture: GlesTexture,
    program: GlesTexProgram,
    fill: (f32, f32, f32, f32),
    border: (f32, f32, f32, f32),
    size: (f32, f32),
    corner_radius: f32,
    inner_offset: f32,
    inner_corner_radius: f32,
    border_px: f32,
    commit: CommitCounter,
}

impl LabelRenderElement {
    pub fn corner_radius(&self) -> f32 {
        self.corner_radius
    }
}

struct Resources {
    context: ContextId<GlesTexture>,
    texture: GlesTexture,
    label_square: GlesTexProgram,
    label_rounded: GlesTexProgram,
}
#[derive(Default)]
pub struct CardRenderer {
    resources: Option<Resources>,
}
impl CardRenderer {
    pub fn label_element(
        &mut self,
        renderer: &mut GlesRenderer,
        id: Id,
        destination: Rectangle<i32, Physical>,
        rounded: bool,
        rgb: (f32, f32, f32),
        alpha: f32,
    ) -> Result<LabelRenderElement, Box<dyn Error>> {
        self.ensure_resources(renderer)?;
        let resources = self.resources.as_ref().expect("ensured above");
        let program = if rounded {
            resources.label_rounded.clone()
        } else {
            resources.label_square.clone()
        };
        let source = Rectangle::<f64, Logical>::new(
            (0.0, 0.0).into(),
            (
                resources.texture.size().w as f64,
                resources.texture.size().h as f64,
            )
                .into(),
        );
        let base = TextureRenderElement::from_static_texture(
            id,
            resources.context.clone(),
            destination.loc.to_f64(),
            resources.texture.clone(),
            1,
            Transform::Normal,
            Some(alpha.clamp(0.0, 1.0)),
            Some(source),
            Some(destination.size.to_logical(1)),
            None,
            Kind::Unspecified,
        );
        Ok(LabelRenderElement {
            base,
            texture: resources.texture.clone(),
            program,
            fill: (rgb.0, rgb.1, rgb.2, 1.0),
            border: (rgb.0, rgb.1, rgb.2, 1.0),
            size: (destination.size.w as f32, destination.size.h as f32),
            corner_radius: destination.size.h as f32 * 0.32,
            inner_offset: 0.0,
            inner_corner_radius: destination.size.h as f32 * 0.32,
            border_px: 0.0,
            commit: label_commit(
                rounded,
                (rgb.0, rgb.1, rgb.2, 1.0),
                (rgb.0, rgb.1, rgb.2, 1.0),
                destination,
                destination.size.h as f32 * 0.32,
                0.0,
                destination.size.h as f32 * 0.32,
                0.0,
            ),
        })
    }

    pub fn overlay_card_element(
        &mut self,
        renderer: &mut GlesRenderer,
        id: Id,
        destination: Rectangle<i32, Physical>,
        style: OverlayCardStyle,
    ) -> Result<LabelRenderElement, Box<dyn Error>> {
        self.ensure_resources(renderer)?;
        let resources = self.resources.as_ref().expect("ensured above");
        let metrics = card_metrics(destination.size, style.content_radius, style.border_px);
        let program = if metrics.content_radius > 0.0 {
            resources.label_rounded.clone()
        } else {
            resources.label_square.clone()
        };
        let source = Rectangle::<f64, Logical>::new(
            (0.0, 0.0).into(),
            (
                resources.texture.size().w as f64,
                resources.texture.size().h as f64,
            )
                .into(),
        );
        let base = TextureRenderElement::from_static_texture(
            id,
            resources.context.clone(),
            destination.loc.to_f64(),
            resources.texture.clone(),
            1,
            Transform::Normal,
            Some(style.alpha.clamp(0.0, 1.0)),
            Some(source),
            Some(destination.size.to_logical(1)),
            None,
            Kind::Unspecified,
        );
        Ok(LabelRenderElement {
            base,
            texture: resources.texture.clone(),
            program,
            fill: style.fill,
            border: style.border,
            size: (destination.size.w as f32, destination.size.h as f32),
            corner_radius: metrics.outer_radius,
            inner_offset: metrics.inner_offset,
            inner_corner_radius: metrics.inner_radius,
            border_px: metrics.border_width,
            commit: label_commit(
                metrics.content_radius > 0.0,
                style.fill,
                style.border,
                destination,
                metrics.outer_radius,
                metrics.inner_offset,
                metrics.inner_radius,
                metrics.border_width,
            ),
        })
    }

    fn ensure_resources(&mut self, renderer: &mut GlesRenderer) -> Result<(), Box<dyn Error>> {
        let context = renderer.context_id();
        if self
            .resources
            .as_ref()
            .is_some_and(|r| r.context == context)
        {
            return Ok(());
        }
        let texture =
            renderer.import_memory(&[255_u8; 64], Fourcc::Abgr8888, (4, 4).into(), false)?;
        let uniforms = [
            UniformName::new("node_color", UniformType::_4f),
            UniformName::new("fill_color", UniformType::_4f),
            UniformName::new("rect_size", UniformType::_2f),
            UniformName::new("inner_rect_size", UniformType::_2f),
            UniformName::new("inner_rect_offset", UniformType::_2f),
            UniformName::new("corner_radius", UniformType::_1f),
            UniformName::new("inner_corner_radius", UniformType::_1f),
            UniformName::new("border_px", UniformType::_1f),
        ];
        let label_square = renderer.compile_custom_texture_shader(LABEL_SQUARE, &uniforms)?;
        let label_rounded = renderer.compile_custom_texture_shader(LABEL_ROUNDED, &uniforms)?;
        self.resources = Some(Resources {
            context,
            texture,
            label_square,
            label_rounded,
        });
        Ok(())
    }
}
impl Element for LabelRenderElement {
    fn id(&self) -> &Id {
        self.base.id()
    }
    fn current_commit(&self) -> CommitCounter {
        self.commit
    }
    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.base.geometry(scale)
    }
    fn transform(&self) -> Transform {
        self.base.transform()
    }
    fn src(&self) -> Rectangle<f64, Buffer> {
        self.base.src()
    }
    fn opaque_regions(&self, _scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        OpaqueRegions::default()
    }
    fn alpha(&self) -> f32 {
        self.base.alpha()
    }
    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl RenderElement<GlesRenderer> for LabelRenderElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        #[cfg(feature = "smithay-render-cache")] _cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        frame.render_texture_from_to(
            &self.texture,
            src,
            dst,
            damage,
            opaque_regions,
            Transform::Normal,
            self.base.alpha(),
            Some(&self.program),
            &[
                Uniform::new("node_color", self.border),
                Uniform::new("fill_color", self.fill),
                Uniform::new("rect_size", self.size),
                Uniform::new(
                    "inner_rect_size",
                    (
                        (self.size.0 - self.inner_offset * 2.0).max(1.0),
                        (self.size.1 - self.inner_offset * 2.0).max(1.0),
                    ),
                ),
                Uniform::new("inner_rect_offset", (self.inner_offset, self.inner_offset)),
                Uniform::new("corner_radius", self.corner_radius),
                Uniform::new("inner_corner_radius", self.inner_corner_radius),
                Uniform::new("border_px", self.border_px),
            ],
        )
    }

    fn underlying_storage(&self, _renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        None
    }
}
