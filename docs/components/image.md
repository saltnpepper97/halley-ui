# Image and icons

Decode an asset once with `assets::ImageData`, retain it in an `Arc`, and use
`Image`. PNG and JPEG decoding and a minimal SVG icon path are included.

```rust
use halley_ui::{Image, assets::ImageData};
use std::sync::Arc;
let icon = Arc::new(ImageData::from_svg(
    br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="#d65d26"/></svg>"##,
    16,
).unwrap());
let image = Image::new("icon", icon).width(24.0).height(24.0)
    .accessible_label("Configuration saved");
```

The component preserves aspect ratio and centers the image inside its content
rectangle. Software sampling is nearest-neighbor. Asset pixels are retained
as premultiplied RGBA. Raster dimensions are bounded; decoders reject invalid
or excessive data. `from_rgba` accepts already-premultiplied pixels.

The minimal SVG decoder targets vector icons and does not include SVG text.
Give meaningful images an accessible label. Unlabelled images are treated as
decorative in the accessibility tree. There is no icon-font dependency.
