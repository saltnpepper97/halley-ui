# List and ListItem

`List` is a retained, scrollable, single-selection list. Give it a bounded height
and stable item keys. A list takes one Tab stop; its options do not each take a
separate stop. Selection is independent of keyboard focus.

```rust
use halley_ui::{Column, Key, List, ListItem, Rect, TextInput, TextSystem, Theme, UiView};
let mut view = UiView::new("launcher").content(
    Column::new("content")
        .child(TextInput::new("search", "").accessible_label("Search"))
        .child(List::new("results").accessible_label("Applications")
            .width(280.0).height(96.0).selected("terminal")
            .child(ListItem::new("terminal", "Terminal").height(40.0))
            .child(ListItem::new("music", "Music").height(40.0).disabled(true))
            .child(ListItem::new("files", "Files").height(40.0))),
);
let mut text = TextSystem::new();
view.prepare(Rect::new(0.0, 0.0, 400.0, 240.0), &Theme::default(), &mut text)?;
let focus_events = view.focus("search")?;
// The host can route Down to the list while the search field keeps keyboard focus.
let selection_events = view.navigate_list("results", Key::Down)?;
assert_eq!(view.focused(), Some("search"));
assert_eq!(view.selected("results"), Some("files"));
let activation_events = view.activate_selected("results")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Selection and navigation

`selected(key)` seeds the selection when a list key first appears. Rebuilding
with the same list key preserves its current selection. Use
`view.select(list_key, Some(item_key))` to change it programmatically, or
`view.select(list_key, None)` to clear it. The returned `SelectionChanged`
event contains the list key and selected item key. These operations preserve
keyboard focus. Invalid or disabled targets return `InteractionError`.

Up/Down moves one enabled option. Home/End selects the first/last enabled option.
Page Up/Down moves approximately one viewport using actual item positions, so
variable row heights and gaps are supported. Navigation clamps at both ends.
With no selection, Down/Page Down starts at the first enabled option and
Up/Page Up at the last. Empty and entirely disabled lists stay unselected.

A focused list handles these keys directly, and Enter activates its selection.
Applications can route keys from another control through `navigate_list` and
`activate_selected`; preserve the text editor's Home/End behavior and let an
active IME own composition navigation. Selection changes never activate an item
by themselves. Pointer press selects and focuses the list; releasing on the same
enabled option emits `Activate`. `PointerCancel` cancels activation.

`ListItem::action` overrides the default action ID, which is the item key.
`ListItem::child` replaces the painted text with composed labels, images, rows,
or cards. The constructor's label remains its default accessible name. Nested
interactive controls, scroll containers, and lists inside an option are rejected
at preparation, as are options outside a list.

## Focus, identity, and scrolling

`view.focus(key)` requests local focus after preparation. Focusing an option
selects it and focuses its owning list. `view.clear_focus()` cancels composition,
capture, and pending paste. Dispatch the returned `FocusChanged` events through
the same path as normal input. Native window activation remains host-owned.

Selection follows stable keys across reordering. If the selected option is
removed or disabled, selection clears at the next preparation; the toolkit does
not silently select a replacement. Removing the list discards its retained state.
Implicit cleanup during preparation does not emit input events; query `selected`
after rebuilding if application state needs reconciliation.

Selection and focus requests reveal their targets during the next preparation,
using fresh layout geometry and all ancestor scroll clips. This also applies to
initial selection and selection retained across structural changes. Normal wheel
scrolling can move away from the selection without snapping back. Prepare again
before drawing after input or programmatic requests.

`selection_color` overrides the selected row fill; the default is the theme's
pressed-button fill. Selection remains visible while another widget has focus.
AccessKit exposes listbox/option roles, selected state, the active descendant,
scroll ranges, and focus/activation actions. Native screen-reader announcements
still require verification in each consuming application.

Use `List::style` for the viewport and `List::item_style` for shared option
styles. `ListItem::style` supplies individual overrides. State properties merge,
so selected, hovered, and focused appearance can coexist. Explicit selected fill
rules override the `selection_color` fallback. See [themes and styling](../styling.md).
