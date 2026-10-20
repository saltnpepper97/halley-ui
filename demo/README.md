# Internal demo

An unpublished workspace app for developing and reviewing Halley UI. It uses the
actual toolkit controls and software renderer in an opaque native Wayland window.
The library retains no window-system dependency. The demo starts in light mode
with a white background; Light/Dark buttons switch the full application palette.

```sh
cargo run --locked -p halley-ui-demo
cargo run --locked -p halley-ui-demo -- --theme dark
```

The Settings page combines sliders, text editing, checkboxes, an animated switch,
radio options, scrolling, and a review dialog. Components shows normal, disabled,
read-only, invalid, and range-endpoint examples. Hover, press, and Tab through the
controls to inspect their interactive states. State lives in memory only.

## Concrete settings check

Press **Run checks** to replay the settings scenario for the selected theme. It
uses an isolated instance of the same application, then displays its resulting
settings and pass count. This resets the demo's in-memory settings to the test
result. Failures appear in the status area. No files are written by the button.

Run the same scenario for both themes without a display server:

```sh
cargo test --locked -p halley-ui-demo
cargo run --locked -p halley-ui-demo -- --check --output target/demo-checks
```

The 12 checks cover opaque/background pixels, keyboard slider events, dragging
across redraws, continuous values, timed switch motion, checkbox/radio input,
editing and disabled state, theme switching, resize/scroll reveal, modal focus,
gallery navigation, and scaled rendering. The CLI writes labelled PNGs at the
initial, moving-switch, small-window, modal, gallery, and completed stages for
each theme. CI runs this command and retains the screenshots for seven days.
These are review artifacts; assertions cover behavior and rendering invariants,
not a pixel-perfect golden comparison across different fonts or machines.

The window host forwards real pointer/keyboard events to the same app, advances
the same animation clock, and presents the same pixels. Headless checks do not
validate compositor presentation, native IME/clipboard, or screen-reader behavior.
The minimal host currently supports ordinary typing and selection; native
clipboard and IME integration are not wired. Test the actual window after changes
to native input, scaling, or presentation.

## Extending it

- `src/lib.rs`: settings/gallery content, application state, and shared rendering.
- `src/theme.rs`: demo-only light/dark palettes and metrics.
- `src/scenario.rs`: repeatable events and assertions; add cases here so the CLI,
  tests, and Run checks exercise the same journey.
- `src/host.rs`: native window, event forwarding, and frame presentation.

Resize the window, scroll the panels, and drag controls while switching focus to
investigate visual or input problems. Keep demo-specific styling here rather
than changing toolkit defaults to make a screenshot look better. The footer's
separate label view lets value readouts update without rebuilding an active
slider's content and cancelling its capture.

Normal `cargo test`/`cargo package` at the repository root still target the
library. The demo is `publish = false` and excluded from the library package.
It writes no application log files or saved preferences. Native errors go to
stderr when run from a terminal.
