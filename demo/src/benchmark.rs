//! Repeatable warm-frame timings; excludes native presentation and font startup.
use crate::{Appearance, Demo, Result};
use halley_ui::Key;
use std::{hint::black_box, time::Instant};

fn report(label: &str, mut milliseconds: Vec<f64>) {
    milliseconds.sort_by(f64::total_cmp);
    let median = milliseconds[milliseconds.len() / 2];
    let p95 = milliseconds[(milliseconds.len() * 95 / 100).min(milliseconds.len() - 1)];
    println!("  {label}: median {median:.3} ms, p95 {p95:.3} ms");
}
pub fn run() -> Result<()> {
    for mode in [Appearance::Light, Appearance::Dark] {
        let mut demo = Demo::new(mode)?;
        demo.render()?;
        demo.focus("balance")?;
        let mut inputs = Vec::new();
        let mut frames = Vec::new();
        for index in 0..48 {
            let before = Instant::now();
            demo.key(if index % 2 == 0 {
                Key::Right
            } else {
                Key::Left
            })?;
            inputs.push(before.elapsed().as_secs_f64() * 1000.0);
            let before = Instant::now();
            black_box(demo.render()?.pixels());
            frames.push(before.elapsed().as_secs_f64() * 1000.0);
        }
        let mut cached = Vec::new();
        for _ in 0..48 {
            let before = Instant::now();
            black_box(demo.render()?.pixels());
            cached.push(before.elapsed().as_secs_f64() * 1000.0);
        }
        println!("{} · 800×1040 · 48 warm samples", mode.name());
        report("input", inputs);
        report("changed frame", frames);
        report("unchanged frame", cached);
    }
    Ok(())
}
