# Toolkit and application boundaries

`halley-ui` is reusable presentation infrastructure. It owns layout, components,
style resolution, text shaping and editing, hit testing, retained interaction,
software drawing, and accessibility semantics. Its public events describe UI
interactions rather than compositor operations.

Applications own their domain data and decisions. Lift discovers applications,
ranks search results, loads icons, chooses result sections, interprets launcher
shortcuts, reads its configuration, and executes selected actions. An `ActionId`
is an application-defined identifier; the toolkit does not resolve it into a
Halley command. Generic editor movement and selection belong in the toolkit;
Lift translates its shortcut conventions into those editor events.

The host owns its event loop, clock, Wayland connection, surfaces, buffers,
clipboard and IME transport, and GPU integration. It supplies input and interprets
`UiEvent` results. `UiStack` manages overlays within the supplied UI bounds;
it cannot observe pointer input outside a host surface or dismiss another app.

Halley's public `halley-api` SDK owns compositor commands and observations,
including optional surface-bound protocol bindings. Applications call it directly.
The toolkit must not depend on `halley-api`, `halley-ipc`, `halley-core`,
`halley-config`, or Smithay. Halley owns focus policy, desktop input routing,
window management, cluster creation, and compositor rendering.

For outside-click dismissal, Lift opts its layer surface into the SDK's public
Wayland interface and decides to close when notified. Halley detects an outside
press and keeps routing input normally. No application namespace checks or
compositor-control calls belong in this toolkit. The same division applies to
future panels and desktop applications.
