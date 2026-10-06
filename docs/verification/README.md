# Verification

## Current 0.1.0 package

The published core has no Smithay or Wayland dependency. GPU renderers,
textures, shaders, damage policy, and protocol buffer handling stay in Halley.
The final core-only integration matched the same 252 historical frame fixtures.
Shared text services use the host's existing glyph texture cache; simple
notification padding uses cached Taffy measurements.

Toolkit checks cover layout, clipping, software pixels/stride, assets, editing,
selection, clipboard ownership, composition, focus, scrolling, and AccessKit tree
validity. README/input/rendering/text-input examples run as documentation tests.
Linux AT-SPI reads, edits, activation, and focus round trips pass using a private
bus and memory-only settings. A full interactive screen-reader/IME application
journey is outside that test's scope. See `../accessibility.md` for reproduction.

## Historical extraction evidence

The notes below describe the original combined-core/GPU prototype before its
package boundary was split. Archived GPU code and tests are not supported
features of the published package. Historical performance measurements are
bounded evidence for that earlier installed build, not a fresh benchmark of all
0.1.0 interaction additions. Commands referring to `gles` require the archived
prototype and must not be run against the current core package.


## Source and test results

The Halley renderer baseline was recorded before extraction, from commit
`037fa122ce76b173f041dcb07ffb0fa2d8ecedc3` plus the comparison test harness.
The preserved fingerprints are in `baseline-frames.txt`; do not overwrite them
with frames from the migrated renderer.

The migration comparison covers 252 frame fingerprints: two font sizes (11,
18), six notification anchors, notifications/screenshot previews/zoom
indicators, and seven animation timestamps. All fingerprints matched after the
extraction and notification layout migration. This is a fixed-fixture image
comparison on software GLES, not a test of every compositor feature.

To compare from the sibling Halley directory on the original font and renderer
installation:

```sh
LIBGL_ALWAYS_SOFTWARE=1 \
HALLEY_UI_PARITY_BASELINE=../halley-ecosystem/halley-ui/docs/verification/baseline-frames.txt \
HALLEY_UI_PARITY_MODE=compare \
cargo test --bin halley ui_library_migration_matches_recorded_frames -- --ignored --test-threads=1
```

The fingerprints use Rust's `DefaultHasher`; a different Rust version, font
installation, or renderer environment can change them independently of the
migration. These fingerprints are same-environment evidence, not portable image
fixtures.

The toolkit's separate software-GLES test verifies stable text identities across
content changes, pixel agreement between incremental and full rendering, no
new damage on a settled frame, and distinct identities across outputs:

```sh
LIBGL_ALWAYS_SOFTWARE=1 cargo test --all-features --test gles_render -- --ignored
```

Toolkit layout, assets, premultiplied pixels, borrowed buffer stride, clipping,
actions, cache invalidation, font suffix parsing, and text padding checks pass.
The README quick start is also compiled and run as a documentation test.
Toolkit strict Clippy passes. Halley's normal workspace tests passed.

## Repaint test alignment and existing lint failures

Before extraction, the ignored
`local_animation_pixels_match_full_repaint_with_blur_shadows_and_reused_buffers`
test failed because it still expected selective repaint after Halley deliberately
switched to conservative rendering with unmodified Smithay. It has been replaced
by `conservative_animation_pixels_match_full_repaint_with_blur_shadows_and_reused_buffers`.
The corrected harness uses the host's conservative blur wrapper and buffer-age
reset policy, verifies full repaint when required, and retains independent
reference pixels, buffer reuse, and settled-frame checks. Its 732 comparisons
pass. The screenshot-preview harness now uses the same current backend policy.
Runtime repaint policy has not been changed by these test corrections.

Halley's strict workspace Clippy run reports existing warnings in untouched
files: `ext_workspace.rs`, `ext_workspace_protocol.rs`, `backend/tty/mod.rs`,
`xwayland/selection.rs`, `session/pointer/mod.rs`, and `wayland/text_input.rs`.
These prevent claiming a clean strict workspace lint run.

## Performance status

The compositor still uses its native Smithay GLES scene, extracted card shaders,
and existing glyph texture cache. Notification layout reuses cached glyph
measurements and caches the Taffy result. It introduces no software full-frame
UI upload or second notification text rasterization.

The installed feature build is now confirmed running. A thirty-second live
CPU/GPU/memory sample and a matched warm-notification benchmark on the real AMD
GPU show no material regression in the observed scope. See the
[performance report](performance.md) for numbers, protocol, and limitations.
Whole-compositor frame-time parity remains unmeasured; pixel agreement and these
bounded measurements do not prove performance parity across all workloads.
