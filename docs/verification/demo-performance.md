# Internal demo rendering performance

Measured October 20, 2026 on an AMD Ryzen 9 5900X. The same warm-frame benchmark
ran in both themes at 800×1040, with 48 samples per measurement. Font startup and
native presentation were excluded. The original demo used an unoptimized dev
build and repainted on every input event, including unchanged pointer motion.

| Theme / measurement | Before median | After median | After p95 |
| --- | ---: | ---: | ---: |
| Light input | 0.106 ms | 0.034 ms | 0.038 ms |
| Light changed frame | 105.877 ms | 9.919 ms | 10.257 ms |
| Dark input | 0.100 ms | 0.034 ms | 0.050 ms |
| Dark changed frame | 106.444 ms | 9.891 ms | 10.125 ms |

Unchanged frames previously took approximately 106 ms. Cached calls now complete
below the benchmark's 0.001 ms display precision. The host does not request a
redraw when an input event leaves the same visual state; an explicit window
redraw can still present the cached pixels. Switch motion presents its final
frame and then stops scheduling work. Run checks executes on a worker thread.

The dev profile now uses optimization level 1, with the toolkit's software drawing
package at level 3. Debug information and overflow checks remain enabled. Both
build settings and redundant work contributed to the improvement; this does not
claim a tenfold algorithmic improvement in the library renderer.

Reproduce the current measurements with:

```sh
cargo run --locked -p halley-ui-demo -- --benchmark
```

These timings measure CPU input handling and software frame generation, not
delivered frame rate or end-to-end interaction latency. Native buffer conversion,
Wayland presentation, compositor work, and display refresh are outside the
benchmark. The full settings scenario and redraw/cache regressions also pass;
the timing values are evidence, not machine-independent CI thresholds.
