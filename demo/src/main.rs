mod host;
use halley_ui_demo::{Appearance, Result, benchmark, scenario};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut appearance = Appearance::Light;
    let mut check = false;
    let mut bench = false;
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--theme" => {
                appearance = match args.next().as_deref() {
                    Some("light") => Appearance::Light,
                    Some("dark") => Appearance::Dark,
                    _ => return Err("--theme requires light or dark".into()),
                }
            }
            "--check" => check = true,
            "--benchmark" => bench = true,
            "--output" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("--output requires a directory")?,
                ))
            }
            "--help" | "-h" => {
                println!(
                    "halley-ui-demo [--theme light|dark]\nhalley-ui-demo --check [--output DIRECTORY]\nhalley-ui-demo --benchmark\nChecks and warm-frame benchmarks run both themes without a display server."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    if bench {
        if check || output.is_some() {
            return Err("--benchmark cannot be combined with --check or --output".into());
        }
        return benchmark::run();
    }
    if check {
        for mode in [Appearance::Light, Appearance::Dark] {
            let report = scenario::run(mode, output.as_deref())?;
            println!("{}: {} checks passed", mode.name(), report.checks.len());
            for name in report.checks {
                println!("  {name}");
            }
        }
        return Ok(());
    }
    if output.is_some() {
        return Err("--output requires --check".into());
    }
    host::run(appearance)
}
