//! Lower prepared widgets into damage-tracked Smithay elements, without offscreen UI copies.
use super::{CardRenderer, OverlayCardStyle, UiTextElement, UiTextRenderer, ids::OutputElementIds};
use crate::{
    Font, Rect,
    assets::ImageData,
    ui::{PaintItem, PreparedView},
};
#[cfg(feature = "smithay-render-cache")]
use smithay::utils::user_data::UserDataMap;
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            ContextId, ImportMem, Renderer,
            element::{
                Element, Id, Kind, RenderElement, UnderlyingStorage, render_elements,
                texture::TextureRenderElement, utils::CropRenderElement,
            },
            gles::{GlesError, GlesFrame, GlesRenderer, GlesTexture},
            utils::{CommitCounter, DamageSet, OpaqueRegions},
        },
    },
    utils::{Buffer, Physical, Rectangle, Scale, Transform},
};
use std::{
    error::Error,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

render_elements! {
    pub UiRenderElement<=GlesRenderer>;
    Card=CropRenderElement<super::LabelRenderElement>,
    Text=CropRenderElement<UiTextElement>,
    Image=CropRenderElement<UiImageElement>,
}
struct ImageEntry {
    data: Arc<ImageData>,
    texture: GlesTexture,
}
pub struct UiRenderer {
    pub text: UiTextRenderer,
    cards: CardRenderer,
    ids: OutputElementIds<(String, u8)>,
    context: Option<ContextId<GlesTexture>>,
    images: Vec<ImageEntry>,
}
impl Default for UiRenderer {
    fn default() -> Self {
        Self::new(&Font::default())
    }
}
impl UiRenderer {
    pub fn new(font: &Font) -> Self {
        Self {
            text: UiTextRenderer::new(font),
            cards: CardRenderer::default(),
            ids: OutputElementIds::default(),
            context: None,
            images: Vec::new(),
        }
    }
    /// Prepare Taffy layout through the GPU glyph cache, without a second CPU raster.
    pub fn prepare<'a>(
        &mut self,
        renderer: &mut GlesRenderer,
        view: &'a mut crate::UiView,
        bounds: Rect,
        theme: &crate::Theme,
    ) -> Result<&'a PreparedView, Box<dyn Error>> {
        let mut measurement_error = None;
        view.prepare_with_color_measure(bounds, theme, |font, value, color| {
            match self
                .text
                .measure_for_font(renderer, font, value, color.bytes())
            {
                Ok(size) => size,
                Err(error) => {
                    measurement_error = Some(error);
                    crate::Size::default()
                }
            }
        })
        .map(|_| ())?;
        if let Some(error) = measurement_error {
            view.invalidate();
            return Err(error);
        }
        Ok(view.prepared().expect("layout prepared"))
    }

    pub fn forget_output(&mut self, output: &str) {
        self.ids.remove_output(output);
    }
    /// Returns front-to-back elements, matching Smithay's scene convention.
    pub fn elements(
        &mut self,
        renderer: &mut GlesRenderer,
        output: &str,
        view: &PreparedView,
        opacity: f32,
    ) -> Result<Vec<UiRenderElement>, Box<dyn Error>> {
        self.ids.advance(output);
        self.text.begin_scene();
        if self.context.as_ref() != Some(&renderer.context_id()) {
            self.images.clear();
            self.context = Some(renderer.context_id());
        }
        let alpha = opacity.clamp(0.0, 1.0);
        let mut elements = Vec::new();
        if alpha <= 0.001 {
            return Ok(elements);
        }
        for item in &view.items {
            match item {
                PaintItem::Card {
                    key,
                    rect,
                    clip,
                    style,
                } => {
                    let destination = physical(*rect);
                    if destination.size.w <= 0 || destination.size.h <= 0 {
                        continue;
                    }
                    let tuple = |color: crate::Color| {
                        let c = color.premultiplied();
                        (c[0], c[1], c[2], c[3])
                    };
                    let id = self.ids.for_output(output).id((key.clone(), 0));
                    let card = self.cards.overlay_card_element(
                        renderer,
                        id,
                        destination,
                        OverlayCardStyle {
                            content_radius: style.radius,
                            border_px: style.border_width,
                            fill: tuple(style.fill),
                            border: tuple(style.border),
                            alpha,
                        },
                    )?;
                    if let Some(clipped) =
                        CropRenderElement::from_element(card, 1.0, physical(*clip))
                    {
                        elements.push(UiRenderElement::Card(clipped));
                    }
                }
                PaintItem::Text {
                    key,
                    rect,
                    clip,
                    text,
                    font,
                    color,
                } => {
                    let destination = physical(*rect);
                    let id = self.ids.for_output(output).id((key.clone(), 1));
                    if let Some(text) = self.text.element_for_font(
                        renderer,
                        id,
                        destination.loc,
                        text,
                        color.bytes(),
                        alpha * color.a,
                        font,
                    )? && let Some(clipped) =
                        CropRenderElement::from_element(text.element, 1.0, physical(*clip))
                    {
                        elements.push(UiRenderElement::Text(clipped));
                    }
                }
                PaintItem::Image {
                    key,
                    rect,
                    clip,
                    data,
                } => {
                    let destination = physical(*rect);
                    if destination.size.w <= 0 || destination.size.h <= 0 {
                        continue;
                    }
                    let texture = if let Some(entry) = self
                        .images
                        .iter()
                        .find(|entry| Arc::ptr_eq(&entry.data, data))
                    {
                        entry.texture.clone()
                    } else {
                        let bytes: usize = self
                            .images
                            .iter()
                            .map(|entry| entry.data.pixels().len())
                            .sum();
                        if self.images.len() >= 64 || bytes + data.pixels().len() > 32 * 1024 * 1024
                        {
                            self.images.clear();
                        }
                        let texture = renderer.import_memory(
                            data.pixels(),
                            Fourcc::Abgr8888,
                            (data.width() as i32, data.height() as i32).into(),
                            false,
                        )?;
                        self.images.push(ImageEntry {
                            data: data.clone(),
                            texture: texture.clone(),
                        });
                        texture
                    };
                    let id = self.ids.for_output(output).id((key.clone(), 2));
                    let mut hasher = DefaultHasher::new();
                    data.id().hash(&mut hasher);
                    let base = TextureRenderElement::from_static_texture(
                        id,
                        renderer.context_id(),
                        destination.loc.to_f64(),
                        texture,
                        1,
                        Transform::Normal,
                        Some(alpha),
                        None,
                        Some(destination.size.to_logical(1)),
                        None,
                        Kind::Unspecified,
                    );
                    let image = UiImageElement {
                        base,
                        commit: CommitCounter::from(hasher.finish() as usize),
                    };
                    if let Some(clipped) =
                        CropRenderElement::from_element(image, 1.0, physical(*clip))
                    {
                        elements.push(UiRenderElement::Image(clipped));
                    }
                }
            }
        }
        elements.reverse();
        Ok(elements)
    }
}
fn physical(rect: Rect) -> Rectangle<i32, Physical> {
    Rectangle::new(
        (rect.origin.x.round() as i32, rect.origin.y.round() as i32).into(),
        (
            rect.size.width.round().max(0.0) as i32,
            rect.size.height.round().max(0.0) as i32,
        )
            .into(),
    )
}
#[derive(Debug)]
pub struct UiImageElement {
    base: TextureRenderElement<GlesTexture>,
    commit: CommitCounter,
}
impl Element for UiImageElement {
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
    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        if commit == Some(self.commit) {
            DamageSet::default()
        } else {
            DamageSet::from_slice(&[Rectangle::from_size(self.geometry(scale).size)])
        }
    }
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.base.opaque_regions(scale)
    }
    fn alpha(&self) -> f32 {
        self.base.alpha()
    }
    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}
impl RenderElement<GlesRenderer> for UiImageElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        #[cfg(feature = "smithay-render-cache")] cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        <TextureRenderElement<GlesTexture> as RenderElement<GlesRenderer>>::draw(
            &self.base,
            frame,
            src,
            dst,
            damage,
            opaque_regions,
            #[cfg(feature = "smithay-render-cache")]
            cache,
        )
    }
    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        <TextureRenderElement<GlesTexture> as RenderElement<GlesRenderer>>::underlying_storage(
            &self.base, renderer,
        )
    }
}
