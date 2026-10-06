#![cfg(feature = "gles")]
use halley_ui::gles::UiRenderer;
use halley_ui::{Anchor, Card, Label, Rect, Theme, UiView};
use smithay::{
    backend::{
        allocator::Fourcc,
        egl::{EGLContext, EGLDisplay, native::EGLSurfacelessDisplay},
        renderer::{
            Bind, Color32F, ExportMem, Offscreen,
            damage::OutputDamageTracker,
            element::Element,
            gles::{GlesRenderer, GlesTexture},
        },
    },
    utils::{Rectangle, Transform},
};

#[test]
#[ignore = "requires surfaceless GLES; run with LIBGL_ALWAYS_SOFTWARE=1 and --ignored"]
fn stable_text_slots_damage_content_changes_and_settled_frames_stop_damaging() {
    let display = unsafe { EGLDisplay::new(EGLSurfacelessDisplay) }.unwrap();
    let context = EGLContext::new(&display).unwrap();
    let mut renderer = unsafe { GlesRenderer::new(context) }.unwrap();
    let mut ui = UiRenderer::default();
    let mut view = UiView::new("notice").anchor(Anchor::TopLeft);
    let theme = Theme::default();
    let mut tracker = OutputDamageTracker::new((180, 80), 1.0, Transform::Normal);
    let mut target: GlesTexture = renderer
        .create_buffer(Fourcc::Abgr8888, (180, 80).into())
        .unwrap();
    let mut reference: GlesTexture = renderer
        .create_buffer(Fourcc::Abgr8888, (180, 80).into())
        .unwrap();
    let mut previous = None;
    for (index, value) in ["AAAA", "BBBB", "BBBB"].into_iter().enumerate() {
        view.set_content(
            Card::new("card")
                .padding(12.0)
                .child(Label::new("message", value)),
        );
        let scene = ui
            .prepare(
                &mut renderer,
                &mut view,
                Rect::new(0.0, 0.0, 180.0, 80.0),
                &theme,
            )
            .unwrap();
        let elements = ui.elements(&mut renderer, "output-a", scene, 1.0).unwrap();
        let current = elements[0].id().clone();
        if let Some(previous) = previous {
            assert_eq!(
                current, previous,
                "text slot identity must survive content changes"
            );
        }
        previous = Some(current);
        let mut fbo = renderer.bind(&mut target).unwrap();
        let result = tracker
            .render_output(
                &mut renderer,
                &mut fbo,
                usize::from(index != 0),
                &elements,
                Color32F::BLACK,
            )
            .unwrap();
        result.sync.wait().unwrap();
        let damaged = result.damage.is_some_and(|d| !d.is_empty());
        assert_eq!(damaged, index < 2);
        let map = renderer
            .copy_framebuffer(
                &fbo,
                Rectangle::from_size((180, 80).into()),
                Fourcc::Abgr8888,
            )
            .unwrap();
        let actual = renderer.map_texture(&map).unwrap().to_vec();
        let mut reference_fbo = renderer.bind(&mut reference).unwrap();
        OutputDamageTracker::new((180, 80), 1.0, Transform::Normal)
            .render_output(
                &mut renderer,
                &mut reference_fbo,
                0,
                &elements,
                Color32F::BLACK,
            )
            .unwrap()
            .sync
            .wait()
            .unwrap();
        let map = renderer
            .copy_framebuffer(
                &reference_fbo,
                Rectangle::from_size((180, 80).into()),
                Fourcc::Abgr8888,
            )
            .unwrap();
        assert_eq!(
            actual,
            renderer.map_texture(&map).unwrap(),
            "incremental rendering must match full repaint"
        );
        let other = ui.elements(&mut renderer, "output-b", scene, 1.0).unwrap();
        assert_ne!(elements[0].id(), other[0].id());
    }
}
