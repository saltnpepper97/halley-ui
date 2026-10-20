# Popovers, menus, and modal dialogs

`UiStack` composes a retained base `UiView` and independently retained overlays
into one `PreparedView`. Draw that snapshot with the software renderer or your
existing renderer. The host still owns its native window, event loop, clipboard,
input method, and application actions. These overlays live inside that window;
they do not create native windows or Wayland popup surfaces.

## A menu followed by a confirmation

```rust
use halley_ui::{
    ActionId, Button, Dialog, InputEvent, Key, LayerEvent, LayerId, Menu,
    MenuItem, Modifiers, OverlayAnchor, OverlayError, Rect, TextSystem, Theme,
    UiEvent, UiStack, UiView,
};

fn dispatch(stack: &mut UiStack, events: Vec<LayerEvent>) -> Result<(), OverlayError> {
    for event in events {
        if let LayerEvent::View { layer, event: UiEvent::Activate(action) } = event {
            match (layer, action.0.as_str()) {
                (LayerId::Base, "actions") => {
                    stack.open(Menu::new("actions-menu", OverlayAnchor::base("actions"))
                        .accessible_label("Item actions")
                        .child(MenuItem::new("delete", "Delete…")
                            .action(ActionId::new("confirm-delete"))))?;
                }
                (LayerId::Overlay(owner), "confirm-delete") if owner == "actions-menu" => {
                    stack.open(Dialog::new("confirmation")
                        .accessible_label("Delete this item?")
                        .initial_focus("cancel")
                        .child(Button::new("cancel", "Cancel"))
                        .child(Button::new("delete", "Delete")))?;
                }
                // Handle confirmation actions in the application, then close the dialog.
                _ => {}
            }
        }
    }
    Ok(())
}

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut stack = UiStack::new(UiView::new("app")
    .content(Button::new("actions", "Actions")));
let mut text = TextSystem::new();
let theme = Theme::default();
let bounds = Rect::new(0.0, 0.0, 640.0, 400.0);
stack.prepare(bounds, &theme, &mut text)?;
stack.focus(&LayerId::Base, "actions")?;
let enter = || InputEvent::KeyDown { key: Key::Enter, modifiers: Modifiers::default() };
let events = stack.handle_event(enter());
dispatch(&mut stack, events)?;
stack.prepare(bounds, &theme, &mut text)?;
let events = stack.take_events();
dispatch(&mut stack, events)?;
let events = stack.handle_event(enter());
dispatch(&mut stack, events)?;
let frame = stack.prepare(bounds, &theme, &mut text)?;
assert!(frame.semantics.iter().any(|node| node.modal));
assert!(stack.is_open("confirmation"));
# Ok(())
# }
```

Menu activation closes the menu **before** delivering its action, so the host can
open the confirmation immediately. Dialog buttons stay open by default; use
`close`, `close_top`, or opt into `close_on_activate(true)`. Drain `take_events`
after preparing to receive focus restoration and anchor-loss notifications.
Dispatch input events and queued preparation events through the same app handler.

## Placement and content

Anchor `Popover` or `Menu` to a widget in the base view, a widget in an earlier
overlay (`OverlayAnchor::overlay`), or an explicit window-coordinate rectangle.
Prepare before opening so widget anchors have current geometry. Missing or fully
clipped anchors are rejected. An open overlay follows its anchor when the window
or content changes; losing the anchor closes it and any overlays above it.

`Placement` supports above/below/left/right with start/end alignment, plus center.
The default is `BelowStart` with a four-pixel gap. Placement flips when the
preferred side overflows and the opposite side fits, then clamps to the window's
available area. The edge margin follows `theme.spacing.medium`. Dialogs are centered. Explicit dimensions
and maximum dimensions remain constrained by the available viewport. Oversized
menus and dialog/popover bodies scroll, and focused controls reveal themselves.

Menu items share `ListItem` builders, including disabled options, actions, custom
noninteractive content, and styles. Up/Down wrap through enabled options;
Home/End and PageUp/PageDown navigate and reveal. Pointer hover selects an option.
Enter activates immediately; Space activates on release if its target still
matches. Empty or entirely disabled menus do not activate anything. This initial
API supplies single-level menus; automatic submenus and typeahead are deferred.

## Input, dismissal, and focus

The front layer owns all pointer, keyboard, and accessibility input. Tab and
Shift+Tab cycle within it; background views retain their local state and focus.
Opening a layer cancels background capture, paste, and composition. Closing it
restores the saved opener if that control still exists and is enabled. Otherwise
focus clears. Closing a parent also closes every layer above it, from front to
back. `initial_focus` chooses a control or list option; the fallback is the first
enabled control, or the overlay's semantic root when there are no controls.

Popovers and menus dismiss on outside press, Escape, and native window blur by
default. The outside press and its release are consumed. Dialogs dismiss on
Escape, block outside input, and survive window blur. Each policy has a
`dismiss_on_*` override. An editor's active IME composition consumes the first
Escape. A dialog adds a backdrop and exposes modal dialog semantics.

After opening, closing, updating, or mutating `base_mut()`, prepare before routing
geometry-dependent input. Normal pointer/key sequences may arrive between
redraws. `needs_prepare()` reports retained changes; the host must also prepare
when its viewport or external theme changes. If painting a translated combined
snapshot, use `handle_event_in` and `ime_state_in` with that same snapshot. Old
snapshots are rejected after structural changes or replacement preparation.

Every `LayerEvent::View` carries its originating `LayerId`; widget keys in the
event remain local to that view. Preserve the layer ID and request number from
`ClipboardRead`, and return its `Paste` through `handle_event_for`. Closed,
inactive, or replaced requests cannot paste into another layer. Forward native
`WindowFocus` events even when the stack is awaiting preparation.

## Retention, styling, and accessibility

`update` replaces an open definition while preserving editor and list state for
surviving keys. Initial text and menu selection are seeds; rebuilding a definition
does not overwrite retained state. `set_text_value` performs a later programmatic
editor change. `base_mut` exposes existing base-view APIs, and `view` provides
read-only layer state. Each view has an independent widget-key scope; overlay
keys themselves must be unique while open. The generated chrome reserves
`<overlay-key>/surface`, `/viewport`, and `/body` within that overlay.

`Theme::overlays` configures optional popover/menu/dialog surfaces, padding, and
the modal backdrop. Surfaces inherit `Theme::card`. Default padding uses
`spacing.small` for menus and `spacing.large` for popovers/dialogs. Definition
builders override those defaults. Menu control and item rules use
`Theme::styles.menu` and `menu_item` independently of ordinary lists. Existing
button/editor rules apply inside dialog and popover content.

Combined paint/geometry keys are qualified by layer. Use `snapshot(&LayerId)` to
inspect the last prepared layer with its local keys and placed coordinates.
Only the active layer's input and semantic nodes appear in the combined frame.
Publish that frame to one `AccessibilityTree` or `UnixBridge`, and route its
actions through `UiStack::handle_accessibility`. Menus expose menu/menu-item
roles; dialogs expose their accessible label and modal state. Background actions
are rejected. Native screen-reader and input-method validation remains the
consuming application's responsibility.
