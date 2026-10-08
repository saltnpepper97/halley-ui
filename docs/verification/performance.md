# Halley UI v0 performance check

Measured on October 5, 2026, after restarting into the installed UI feature
build. The running executable checksum matched both the installed executable
and optimized release artifact.

## Live desktop sample

Thirty one-second intervals, with normal desktop/voice activity. This was not an
idle baseline or a controlled comparison with the old desktop session.

| Measurement | Result |
| --- | --- |
| Halley CPU, mean | 13.40% of one CPU core |
| Halley CPU, highest one-second interval | 16.95% of one CPU core |
| Halley graphics engine, mean | 2.94% |
| Halley graphics engine, highest interval | 3.56% |
| CPU resident memory, start and end | 139.41 MiB, unchanged |
| Driver-reported VRAM, start and end | 333.62 MiB, unchanged |

CPU comes from process CPU-time deltas, not the lifetime average reported by
`ps`. GPU activity comes from this process's AMD DRM engine counters, with
multiple file descriptors for the same DRM client deduplicated. GPU memory
includes scene/client buffers and is not attributable solely to Halley UI.
The monitors were configured for approximately 180 Hz and 75 Hz; these are
configured refresh rates, not measured delivered frame rates.

## Matched warm-notification benchmark

The benchmark runs both paths in one optimized executable on the real AMD GPU.
It compares the pre-extraction text renderer from Halley's base commit with the
shared text renderer and cached [Taffy](https://github.com/DioxusLabs/taffy)
notification padding. The card renderer and its shaders are held constant.
Both paths produce byte-identical pixels.
Dependency package versions match Halley's installed-build lockfile.

Each of eight trials runs 20,000 cached scene preparations, then 200 timed draws
following warmup. Legacy/shared order alternates between trials. Timing excludes
font initialization, first glyph upload, and shader compilation. Draw timing
includes CPU preparation, submission, and waiting for GPU completion, but no
pixel readback. Both paths force a full draw of the same 384 by 128 target.

| Measurement, median across trials | Legacy | Shared UI |
| --- | --- | --- |
| Cached scene preparation | 0.578 microseconds | 0.654 microseconds |
| Median draw and completion | 0.07346 ms | 0.07335 ms |
| 95th percentile draw and completion | 0.11736 ms | 0.11160 ms |

The added cached preparation work is about **0.076 microseconds per notice**.
Draw times are similar; these measurements do not justify claiming a speedup.
[Raw trial results](notification-performance.txt) are retained alongside this
report. The standalone harness, its fixed legacy source, dependency lockfile,
and live samples are archived locally under
`~/.local/state/halley-ui/verification/ui-v0-2026-10-05/`.

## Conclusion and limits

The observed live session and this targeted notification benchmark show no
material performance regression. Memory stayed flat during the short live
sample. This does not establish long-term leak freedom, cold-rendering parity,
whole-compositor frame-time parity, or performance under every UI workload.
Full-output blur, client composition, multi-output presentation, and input
latency were not included in the isolated notification benchmark.
