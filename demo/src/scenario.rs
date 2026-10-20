//! One concrete interaction journey, used by tests, the CLI, and Run checks.
use crate::{Appearance, Demo, Page, Result};
use halley_ui::{InputEvent, Key, LayerEvent, LayerId, Modifiers, Point, UiEvent, ui::ControlKind};
use std::{fs, path::Path, time::Duration};

pub struct Report {
    pub checks: Vec<&'static str>,
    pub demo: Demo,
}
fn ensure(condition: bool, what: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(format!("scenario failed: {what}").into())
    }
}
fn capture(demo: &mut Demo, output: Option<&Path>, name: &str) -> Result<()> {
    if let Some(path) = output {
        demo.save(&path.join(format!("{}-{name}.png", demo.appearance.name())))?;
    }
    Ok(())
}
fn opaque(demo: &mut Demo) -> Result<()> {
    let background = demo.appearance.palette().background.bytes();
    let pixels = demo.render()?.pixels();
    ensure(
        pixels.chunks_exact(4).all(|p| p[3] == 255),
        "every client pixel is opaque",
    )?;
    ensure(pixels[..3] == background, "correct background color")
}
pub fn run(appearance: Appearance, output: Option<&Path>) -> Result<Report> {
    if let Some(path) = output {
        fs::create_dir_all(path)?;
    }
    let mut demo = Demo::new(appearance)?;
    let mut checks = Vec::new();
    opaque(&mut demo)?;
    capture(&mut demo, output, "settings")?;
    checks.push("opaque background");

    demo.focus("volume")?;
    let events = demo.key(Key::Right)?;
    ensure(
        events.contains(&LayerEvent::View {
            layer: LayerId::Base,
            event: UiEvent::SliderChanged {
                key: "volume".into(),
                value: 70.0,
            },
        }),
        "keyboard changes volume and emits one value event",
    )?;
    checks.push("keyboard slider input");

    demo.render()?;
    let c = demo
        .scene
        .controls
        .iter()
        .find(|c| c.key == "base/volume")
        .ok_or("missing volume")?
        .clone();
    let ControlKind::Slider { thumb_size, .. } = c.kind else {
        return Err("volume is not a slider".into());
    };
    let point = |fraction: f32| {
        Point::new(
            c.content.origin.x + thumb_size * 0.5 + (c.content.size.width - thumb_size) * fraction,
            c.content.origin.y + c.content.size.height * 0.5,
        )
    };
    demo.input(InputEvent::PointerDown {
        position: point(0.9),
        shift: false,
    })?;
    demo.render()?;
    demo.input(InputEvent::PointerMove {
        position: point(0.4),
    })?;
    demo.render()?;
    demo.input(InputEvent::PointerUp {
        position: point(0.4),
    })?;
    ensure(
        demo.stack.base().slider_value("volume") == Some(40.0),
        "drag survives redraws",
    )?;
    checks.push("drag across redraws");
    demo.focus("balance")?;
    demo.key(Key::PageUp)?;
    ensure(
        (demo.stack.base().slider_value("balance").unwrap_or(9.0) - 0.2).abs() < 1e-10,
        "continuous balance adjustment",
    )?;
    checks.push("continuous numeric range");

    demo.focus("autoplay")?;
    demo.key(Key::Space)?;
    demo.input(InputEvent::KeyUp { key: Key::Space })?;
    ensure(
        demo.stack.base().checked("autoplay") == Some(false),
        "switch changes logical value",
    )?;
    ensure(demo.stack.has_animations(), "switch starts motion")?;
    demo.advance(Duration::from_millis(60));
    capture(&mut demo, output, "switch-moving")?;
    demo.advance(Duration::from_millis(100));
    ensure(
        !demo.stack.has_animations(),
        "switch finishes motion without an idle frame loop",
    )?;
    checks.push("switch motion");
    demo.focus("normalize")?;
    demo.key(Key::Space)?;
    demo.input(InputEvent::KeyUp { key: Key::Space })?;
    ensure(
        demo.stack.base().checked("normalize") == Some(true),
        "checkbox toggles",
    )?;
    demo.focus("balanced")?;
    demo.key(Key::Down)?;
    ensure(
        demo.stack.base().radio_selected("preset") == Some("quiet"),
        "radio skips disabled option",
    )?;
    demo.key(Key::Down)?;
    ensure(
        demo.stack.base().radio_selected("preset") == Some("balanced"),
        "radio wraps past disabled option",
    )?;
    checks.push("checkbox and radio input");

    demo.focus("name")?;
    demo.input(InputEvent::KeyDown {
        key: Key::A,
        modifiers: Modifiers {
            control: true,
            ..Default::default()
        },
    })?;
    demo.input(InputEvent::Text("Test speakers".into()))?;
    demo.render()?;
    ensure(
        demo.stack.base().text_value("name") == Some("Test speakers"),
        "edit output name",
    )?;
    let locked = demo
        .scene
        .controls
        .iter()
        .find(|c| c.key == "base/locked")
        .ok_or("missing disabled slider")?;
    ensure(locked.disabled, "locked slider is disabled")?;
    ensure(
        demo.stack.base().slider_value("locked") == Some(30.0),
        "disabled value remains unchanged",
    )?;
    checks.push("text editing and disabled state");

    demo.click(if appearance == Appearance::Light {
        "theme-dark"
    } else {
        "theme-light"
    })?;
    ensure(
        demo.appearance == appearance.opposite(),
        "theme selector changes appearance",
    )?;
    opaque(&mut demo)?;
    demo.click(if appearance == Appearance::Light {
        "theme-light"
    } else {
        "theme-dark"
    })?;
    ensure(
        demo.stack.base().slider_value("volume") == Some(40.0)
            && demo.stack.base().text_value("name") == Some("Test speakers"),
        "theme switch preserves values",
    )?;
    checks.push("light/dark selector and retained values");

    demo.resize(560, 600, 1.0)?;
    demo.render()?;
    demo.focus("name")?;
    ensure(
        demo.scene.scrolls.iter().any(|s| s.offset.y > 0.0),
        "small window reveals focused field",
    )?;
    opaque(&mut demo)?;
    capture(&mut demo, output, "small-window")?;
    demo.resize(800, 860, 1.0)?;
    demo.render()?;
    checks.push("resize and scroll reveal");

    demo.click("review")?;
    ensure(
        demo.stack.is_open("review-dialog"),
        "review opens a real modal",
    )?;
    ensure(
        demo.stack.focus(&LayerId::Base, "volume").is_err(),
        "modal blocks background focus",
    )?;
    capture(&mut demo, output, "modal")?;
    demo.key(Key::Escape)?;
    demo.render()?;
    ensure(
        !demo.stack.is_open("review-dialog") && demo.stack.base().focused() == Some("review"),
        "Escape closes modal and restores focus",
    )?;
    checks.push("modal scope and focus restoration");

    demo.click("components-tab")?;
    ensure(demo.page == Page::Components, "gallery navigation")?;
    ensure(
        demo.scene.scrolls.iter().all(|s| s.offset.y == 0.0),
        "gallery starts at its own top",
    )?;
    ensure(
        demo.scene.semantics.iter().any(|s| s.invalid)
            && demo.scene.semantics.iter().any(|s| s.disabled),
        "gallery contains error and disabled examples",
    )?;
    capture(&mut demo, output, "components")?;
    demo.click("settings-tab")?;
    ensure(
        demo.stack.base().slider_value("volume") == Some(40.0)
            && demo.stack.base().checked("normalize") == Some(true)
            && demo.stack.base().text_value("name") == Some("Test speakers"),
        "settings survive gallery navigation",
    )?;
    checks.push("component gallery and page state");

    demo.resize(1200, 1290, 1.5)?;
    opaque(&mut demo)?;
    ensure(
        demo.stack.base().slider_value("volume") == Some(40.0),
        "display scale retains settings",
    )?;
    demo.resize(800, 860, 1.0)?;
    checks.push("scaled rendering");
    demo.set_status(format!(
        "{} checks passed · {} theme",
        checks.len(),
        appearance.name()
    ));
    capture(&mut demo, output, "passed")?;
    demo.render()?;
    Ok(Report { checks, demo })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_journey_in_both_themes() {
        for mode in [Appearance::Light, Appearance::Dark] {
            let report = run(mode, None).unwrap();
            assert_eq!(report.checks.len(), 12);
            assert_eq!(report.demo.appearance, mode);
        }
    }
    #[test]
    fn theme_button_and_run_button_use_the_real_input_path() {
        let mut demo = Demo::new(Appearance::Light).unwrap();
        demo.render().unwrap();
        demo.click("theme-dark").unwrap();
        assert_eq!(demo.appearance, Appearance::Dark);
        demo.click("run-checks").unwrap();
        assert!(demo.take_run_request());
        assert!(!demo.take_run_request());
    }
}
