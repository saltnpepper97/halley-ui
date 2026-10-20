//! Native window and event translation only. All application behavior lives in Demo.
use halley_ui::{InputEvent, Key, Modifiers, Point};
use halley_ui_demo::{Appearance, Demo, Result, scenario};
use std::{
    num::NonZeroU32,
    rc::Rc,
    sync::mpsc::{self, Receiver},
    thread,
    time::Instant,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{Key as NativeKey, NamedKey},
    platform::wayland::WindowAttributesExtWayland,
    window::{Window, WindowId},
};

struct Host {
    demo: Demo,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    cursor: Point,
    modifiers: Modifiers,
    last_tick: Instant,
    wake: EventLoopProxy<()>,
    checks: Option<Receiver<std::result::Result<scenario::Report, String>>>,
}
impl Host {
    fn tick(&mut self) {
        let now = Instant::now();
        self.demo.advance(now - self.last_tick);
        self.last_tick = now;
    }
    fn draw(&mut self) -> Result<()> {
        self.tick();
        let window = self.window.as_ref().ok_or("missing window")?;
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(());
        };
        self.demo
            .resize(size.width, size.height, window.scale_factor() as f32)?;
        let pixels = self.demo.render()?;
        let surface = self.surface.as_mut().ok_or("missing surface")?;
        surface.resize(width, height)?;
        let mut buffer = surface.buffer_mut()?;
        for (dst, p) in buffer.iter_mut().zip(pixels.pixels().chunks_exact(4)) {
            *dst = (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32;
        }
        buffer.present()?;
        if self.demo.stack.has_animations() {
            window.request_redraw();
        }
        Ok(())
    }
    fn input(&mut self, input: InputEvent) -> Result<()> {
        self.tick();
        let previous_appearance = self.demo.appearance;
        self.demo.input(input)?;
        if self.demo.take_run_request() && self.checks.is_none() {
            let mode = self.demo.appearance;
            let (sender, receiver) = mpsc::channel();
            let wake = self.wake.clone();
            thread::Builder::new()
                .name("demo-checks".into())
                .spawn(move || {
                    let result = std::panic::catch_unwind(|| scenario::run(mode, None))
                        .map_err(|_| "Scenario panicked".to_string())
                        .and_then(|result| result.map_err(|error| error.to_string()));
                    let _ = sender.send(result);
                    let _ = wake.send_event(());
                })?;
            self.checks = Some(receiver);
            self.demo.set_status("Running checks…");
        }
        if previous_appearance != self.demo.appearance {
            self.window
                .as_ref()
                .ok_or("missing window")?
                .set_theme(Some(native_theme(self.demo.appearance)));
        }
        if self.demo.needs_redraw() || self.demo.stack.has_animations() {
            self.window
                .as_ref()
                .ok_or("missing window")?
                .request_redraw();
        }
        Ok(())
    }
}
fn mapped(key: &NativeKey) -> Option<Key> {
    Some(match key {
        NativeKey::Named(NamedKey::Tab) => Key::Tab,
        NativeKey::Named(NamedKey::Space) => Key::Space,
        NativeKey::Named(NamedKey::Enter) => Key::Enter,
        NativeKey::Named(NamedKey::Escape) => Key::Escape,
        NativeKey::Named(NamedKey::ArrowLeft) => Key::Left,
        NativeKey::Named(NamedKey::ArrowRight) => Key::Right,
        NativeKey::Named(NamedKey::ArrowUp) => Key::Up,
        NativeKey::Named(NamedKey::ArrowDown) => Key::Down,
        NativeKey::Named(NamedKey::PageUp) => Key::PageUp,
        NativeKey::Named(NamedKey::PageDown) => Key::PageDown,
        NativeKey::Named(NamedKey::Home) => Key::Home,
        NativeKey::Named(NamedKey::End) => Key::End,
        NativeKey::Named(NamedKey::Backspace) => Key::Backspace,
        NativeKey::Named(NamedKey::Delete) => Key::Delete,
        NativeKey::Character(value) if value.eq_ignore_ascii_case("a") => Key::A,
        NativeKey::Character(value) if value.eq_ignore_ascii_case("c") => Key::C,
        NativeKey::Character(value) if value.eq_ignore_ascii_case("x") => Key::X,
        NativeKey::Character(value) if value.eq_ignore_ascii_case("v") => Key::V,
        _ => return None,
    })
}
impl ApplicationHandler for Host {
    fn user_event(&mut self, _: &ActiveEventLoop, _: ()) {
        let Some(receiver) = self.checks.take() else {
            return;
        };
        let result = receiver
            .try_recv()
            .map_err(|error| error.to_string())
            .and_then(|result| result);
        let applied = (|| -> Result<()> {
            let report = result.map_err(|error| format!("Check failed: {error}"))?;
            let (width, height, scale) = self.demo.viewport();
            let appearance = self.demo.appearance;
            self.demo = report.demo;
            self.demo.set_appearance(appearance);
            self.demo.resize(width, height, scale)?;
            let focused = self
                .window
                .as_ref()
                .is_some_and(|window| window.has_focus());
            self.demo.input(InputEvent::WindowFocus(focused))?;
            Ok(())
        })();
        if let Err(error) = applied {
            self.demo.set_status(error.to_string());
        }
        self.last_tick = Instant::now();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let created = (|| -> Result<_> {
            let window = Rc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Halley UI · Internal demo")
                        .with_name("halley-ui-demo", "halley-ui")
                        .with_theme(Some(native_theme(self.demo.appearance)))
                        .with_transparent(false)
                        .with_inner_size(LogicalSize::new(800.0, 1040.0))
                        .with_min_inner_size(LogicalSize::new(560.0, 560.0)),
                )?,
            );
            let context = softbuffer::Context::new(window.clone())?;
            let surface = softbuffer::Surface::new(&context, window.clone())?;
            Ok((window, surface))
        })();
        match created {
            Ok((window, surface)) => {
                window.request_redraw();
                self.window = Some(window);
                self.surface = Some(surface);
                self.last_tick = Instant::now();
            }
            Err(error) => {
                eprintln!("Window initialization failed: {error}");
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let input = match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw() {
                    eprintln!("Rendering failed: {error}");
                    event_loop.exit();
                }
                return;
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return;
            }
            WindowEvent::Focused(value) => Some(InputEvent::WindowFocus(value)),
            WindowEvent::ModifiersChanged(value) => {
                let m = value.state();
                self.modifiers = Modifiers {
                    shift: m.shift_key(),
                    control: m.control_key(),
                    alt: m.alt_key(),
                };
                None
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Point::new(position.x as f32, position.y as f32);
                Some(InputEvent::PointerMove {
                    position: self.cursor,
                })
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => Some(if state.is_pressed() {
                InputEvent::PointerDown {
                    position: self.cursor,
                    shift: self.modifiers.shift,
                }
            } else {
                InputEvent::PointerUp {
                    position: self.cursor,
                }
            }),
            WindowEvent::MouseWheel { delta, .. } => {
                let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor()) as f32;
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        Point::new(-x * 36.0 * scale, -y * 36.0 * scale)
                    }
                    MouseScrollDelta::PixelDelta(p) => Point::new(-p.x as f32, -p.y as f32),
                };
                Some(InputEvent::Wheel {
                    position: self.cursor,
                    delta,
                })
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                if let Some(key) = mapped(&event.logical_key) {
                    let input = if event.state.is_pressed() {
                        InputEvent::KeyDown {
                            key,
                            modifiers: self.modifiers,
                        }
                    } else {
                        InputEvent::KeyUp { key }
                    };
                    if let Err(error) = self.input(input) {
                        self.demo.set_status(error.to_string());
                    }
                }
                if event.state.is_pressed() && !self.modifiers.control && !self.modifiers.alt {
                    event
                        .text
                        .filter(|s| !s.chars().any(char::is_control))
                        .map(|s| InputEvent::Text(s.to_string()))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(input) = input
            && let Err(error) = self.input(input)
        {
            self.demo.set_status(format!("Input failed: {error}"));
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}
pub fn run(appearance: Appearance) -> Result<()> {
    let event_loop = EventLoop::new()?;
    let wake = event_loop.create_proxy();
    event_loop.run_app(&mut Host {
        demo: Demo::new(appearance)?,
        window: None,
        surface: None,
        cursor: Point::default(),
        modifiers: Modifiers::default(),
        last_tick: Instant::now(),
        wake,
        checks: None,
    })?;
    Ok(())
}

fn native_theme(appearance: Appearance) -> winit::window::Theme {
    match appearance {
        Appearance::Light => winit::window::Theme::Light,
        Appearance::Dark => winit::window::Theme::Dark,
    }
}
