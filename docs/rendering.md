# Rendering and buffers

`UiView::prepare` uses Taffy and shared Cosmic Text / Swash services to produce a
`PreparedView`. Its paint items are in back-to-front order and carry explicit
clips. Geometry is in the coordinate system of the supplied bounds.

## Software drawing

`PixelBuffer` owns tightly packed premultiplied RGBA pixels. `Surface::new`
borrows a host buffer with explicit width, height, stride, and `PixelFormat`.
RGBA and BGRA are supported. Drawing does not copy a complete borrowed buffer.

```rust
use halley_ui::{Color, software::{PixelFormat, Surface}};
let mut pixels = vec![0_u8; 64 * 32 * 4];
let mut surface = Surface::new(&mut pixels, 64, 32, 64 * 4, PixelFormat::Bgra).unwrap();
surface.clear(Color::rgba(0.15, 0.18, 0.22, 1.0));
```

Use `surface.draw(prepared, &mut text, opacity)` after clearing/reconstructing
the appropriate background. The software renderer does not own your frame
scheduler or damage tracker. Borrowed lifetimes prevent drawing into a buffer
after the host releases it. Allocate/reuse SHM or other native buffers in the
application; protocol objects and buffer-release handling remain there.

## Existing host/GPU renderers

Consume `PreparedView::items`: `Card`, `Text`, and `Image` describe primitives.
Keep their keys stable and include clips, style/content changes, and opacity in
your renderer's damage policy. Apply any host translation to both rendering and
input/semantics with `PreparedView::translated`.

`prepare_with_measure` and `prepare_with_color_measure` accept existing cached
glyph measurements. `TextSystem::raster_uncached` supplies glyph pixels without
retaining another CPU raster cache when the host already caches GPU textures.
The core has no Smithay, EGL, GPU-context, or Wayland dependency. Font metrics,
output scaling, surface lifetime, GPU uploads, and renderer compatibility are
explicit host concerns.

Text input geometry uses an additional bounded shaping cache without retaining
pixel copies. Invalidate the view when an external measurement service's font
environment changes. Keep both font services configured consistently.
