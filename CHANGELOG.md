# Changelog

## 0.1.2

- Skip transparent chrome and glyph pixels in the software renderer.
- Use exact straight-edge distance calculations outside rounded corners.
- Clear borrowed buffers by row while preserving stride padding.

## 0.1.1

- Buttons can contain composed widgets while retaining full-row focus, activation, and accessibility semantics.
- Added a custom-content guide and interaction regression test.

## 0.1.0

Initial release of the Smithay-independent toolkit.

- Taffy-backed cards, containers, rows, columns, labels, buttons, and images.
- Single-line Unicode editing, selection, clipboard requests, and IME composition.
- Keyboard and pointer focus, activation, and clipped scrolling with focus reveal.
- Shared Cosmic Text / Swash shaping and glyph rendering with bounded caches.
- Owned and borrowed software buffers, premultiplied RGBA/BGRA, PNG/JPEG/SVG icons.
- Optional AccessKit semantics and Linux AT-SPI bridge.
- Component guides, tested quick starts, and runnable rendering examples.

GPU and Wayland integration remains host-owned; the package has no Smithay dependency.
