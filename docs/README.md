# Halley UI documentation

Start with the [quick start](../README.md), then build a retained `UiView` and
translate your host's input events into the toolkit's event types.

## Components

- [Card and Container](components/card.md): panels, padding, and visual style.
- [Row and Column](components/layout.md): composition, spacing, and sizing.
- [Label](components/label.md): fonts, colors, alignment, and truncation.
- [Button](components/button.md): actions, disabled state, and keyboard activation.
- [TextInput](components/text-input.md): values, selection, clipboard, and composition.
- [Scroll](components/scroll.md): viewports, wheel movement, and focus reveal.
- [Image and icons](components/image.md): decoded assets and aspect ratio.

## Host integration

- [Input and focus](input.md)
- [Rendering and buffers](rendering.md)
- [Accessibility](accessibility.md)

The toolkit owns UI state and produces paint, hit, and semantic geometry. Your
application owns its event loop, display connection, native window, renderer,
clipboard, input method, business actions, and repaint scheduling.

Maintainers can find historical checks in [verification](verification/README.md)
and the deferred GPU migration in [reintegration](reintegration/README.md).
These repository-only archives are not shipped as supported backends.
