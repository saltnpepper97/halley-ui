# Changelog

## Unreleased

- Center checkbox marks with font-independent vector assets.
- Add opt-in, host-clock-driven switch thumb/track motion with smooth reversal,
  an immediate reduced-motion path, and bounded animation lifetimes.

- Add retained Checkbox, Switch, and RadioGroup/RadioOption controls with silent
  programmatic updates, user value-change events, and disabled values.
- Add Space activation, one Tab stop per radio group, wrapping arrow navigation,
  clipping/focus reveal, capture cancellation, and overlay integration.
- Add form metrics, component styling, checked accessibility state, documentation,
  and interaction/rendering regressions. See the form guide for API additions.

- Add renderer-neutral `UiStack`, popovers, menus, and modal dialogs with anchored
  flip/clamp placement, constrained scrolling, scoped input, and focus restoration.
- Add keyboard/hover menu selection, dismissal policies, modal backdrops, and
  layer-aware events with stale clipboard and frame protection.
- Add menu/menu-item/dialog accessibility roles and modal state; extend `Theme`,
  `ComponentStyles`, and `SemanticNode` for overlay styling and semantics.
- Add overlay integration documentation and interaction/rendering regressions.

- Add partial component and widget styles for normal, selected, hover, press,
  focus, error, and disabled states, including list-wide option styles.
- Add shared spacing defaults, typography roles, and configurable editor colors.
- Resolve fonts and padding consistently for layout, custom child text, caret,
  pointer hits, and IME geometry; theme changes preserve retained UI state.
- Expose control validation state through AccessKit.
- Extend `Theme` and `SemanticNode`; replace `PreparedView::button_visuals` with
  state-aware preparation. See the styling guide for API migration.
- Add retained selectable lists with stable item keys, disabled options,
  arrow/page navigation, automatic reveal, and selection separate from focus.
- Add programmatic focus, selection, and selected-item activation APIs.
- Expose AccessKit listbox/option semantics, selection, and active descendants.
- Add list navigation, focus, scrolling, and accessibility regression coverage.
- Add public key, event, control, and semantic enum variants for lists;
  consumers with exhaustive matches must handle the new variants.

## 0.1.2

- Skip transparent chrome and glyph pixels in the software renderer.
- Use exact straight-edge distance calculations outside rounded corners.
- Clear borrowed buffers by row while preserving stride padding.

## 0.1.1

- Buttons can contain composed widgets while retaining full-row focus, activation, and accessibility semantics.
- Added a custom-content guide and interaction regression test.

## 0.1.0

Initial release of the Smithay-independent toolkit.

- [Taffy](https://github.com/DioxusLabs/taffy)-backed cards, containers, rows,
  columns, labels, buttons, and images.
- Single-line Unicode editing, selection, clipboard requests, and IME composition.
- Keyboard and pointer focus, activation, and clipped scrolling with focus reveal.
- Shared Cosmic Text / Swash shaping and glyph rendering with bounded caches.
- Owned and borrowed software buffers, premultiplied RGBA/BGRA, PNG/JPEG/SVG icons.
- Optional AccessKit semantics and Linux AT-SPI bridge.
- Component guides, tested quick starts, and runnable rendering examples.

GPU and Wayland integration remains host-owned; the package has no Smithay dependency.
