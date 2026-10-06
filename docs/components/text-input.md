# TextInput

A retained single-line editor, suitable for naming a cluster. Give it an
accessible name in addition to any visible label or placeholder.

```rust
use halley_ui::{TextInput, UiView};
let mut view = UiView::new("cluster-dialog").content(
    TextInput::new("name", "Work")
        .accessible_label("Cluster name")
        .placeholder("Enter a name")
        .width(280.0),
);
assert!(view.set_text_value("name", "Research"));
assert_eq!(view.text_value("name"), Some("Research"));
```

The constructor seeds state when the key first appears. Rebuilding content with
the same key preserves edits. Use `set_text_value` for programmatic replacement;
it resets the selection and cancels composition and pending paste. Removing a
key discards its retained state on the next preparation/input synchronization.

Editing supports grapheme-safe logical left/right motion, Ctrl-word movement,
Home/End, Shift selection, pointer selection, Ctrl-A, Backspace/Delete,
Ctrl-C/X/V clipboard requests, and Enter submission. UTF-8 byte offsets are used
for selection and surrounding-text APIs. Control characters and hard line
separators are removed from inserted text; multiline editing is unsupported.

Committed text arrives through `InputEvent::Text`. IME preedit is temporary,
underlined, and replaces the selection only on commit. Escape or focus loss
cancels it. `DeleteSurrounding` supports IME deletion in UTF-8 byte counts,
rounded outwards to grapheme boundaries. Obtain candidate-popup geometry with
`ime_state` after preparing, or `ime_state_in` for translated snapshots.

The editor scrolls its text horizontally to keep the caret visible. The caret
is steady in 0.1.0; there is no animation timer. `read_only(true)` allows focus,
selection, and copy while rejecting edits; `disabled(true)` also removes focus
and input. `TextChanged` and `Submit` carry the current committed value.

See [input integration](../input.md) for clipboard replies and event routing.
