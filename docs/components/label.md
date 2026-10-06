# Label

`Label::new(key, text)` displays single-line text with the theme font and text
color. `font`, `color`, `align`, and `overflow` customize it.

```rust
use halley_ui::{Font, Label, TextOverflow};
let title = Label::new("title", "A long cluster name")
    .font(Font { family: "monospace".into(), size: 18 })
    .max_width(160.0)
    .overflow(TextOverflow::EllipsisEnd);
```

The default overflow is `Clip`, which retains the full text and clips painting.
`EllipsisEnd` and `EllipsisMiddle` truncate at grapheme boundaries. If an ellipsis
itself does not fit, the displayed text is empty. Alignment is within the label's
content rectangle; use a width when extra alignment space is needed.

The semantic value contains the full original text, including visually
truncated text. Fonts use Cosmic Text and system font discovery. Family names
can include style suffixes such as `Bold` or `Italic`; reload/invalidate a view
when changing its externally supplied font environment.
