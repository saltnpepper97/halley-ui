//! The interactive host and scripted checks share this application and renderer.
pub mod scenario;
pub mod theme;

use halley_ui::{
    Anchor, Button, Card, Checkbox, Column, Dialog, Font, InputEvent, Key, Label, LayerEvent,
    LayerId, Point, RadioGroup, RadioOption, Rect, Row, Scroll, Slider, Switch, TextInput,
    TextRole, TextSystem, Theme, UiEvent, UiStack, UiView, Widget,
    software::PixelBuffer,
    ui::{Align, PreparedView},
};
use std::{error::Error, path::Path, time::Duration};
pub use theme::Appearance;
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Settings,
    Components,
}

#[derive(Clone)]
struct Settings {
    volume: f64,
    balance: f64,
    autoplay: bool,
    normalize: bool,
    preset: String,
    name: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 65.0,
            balance: 0.0,
            autoplay: true,
            normalize: false,
            preset: "balanced".into(),
            name: "Desk speakers".into(),
        }
    }
}

pub struct Demo {
    pub stack: UiStack,
    pub appearance: Appearance,
    pub page: Page,
    pub scene: PreparedView,
    text: TextSystem,
    footer: UiView,
    theme: Theme,
    settings: Settings,
    width: u32,
    height: u32,
    scale: f32,
    pixels: PixelBuffer,
    status: String,
    run_requested: bool,
}
impl Demo {
    pub fn new(appearance: Appearance) -> Result<Self> {
        let mut demo = Self {
            stack: UiStack::new(UiView::new("demo").anchor(Anchor::TopLeft).margin(0.0)),
            appearance,
            page: Page::Settings,
            scene: PreparedView::default(),
            text: TextSystem::new(),
            footer: UiView::new("status").anchor(Anchor::TopLeft).margin(0.0),
            theme: appearance.theme(1.0),
            settings: Settings::default(),
            width: 800,
            height: 1040,
            scale: 1.0,
            pixels: PixelBuffer::new(800, 1040)?,
            status: "Ready · use Run checks to replay the settings test".into(),
            run_requested: false,
        };
        demo.stack
            .set_switch_animation_duration(Duration::from_millis(160));
        demo.rebuild();
        Ok(demo)
    }
    pub fn viewport(&self) -> (u32, u32, f32) {
        (self.width, self.height, self.scale)
    }
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) -> Result<()> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err("invalid display scale".into());
        }
        if self.viewport() == (width, height, scale) {
            return Ok(());
        }
        self.width = width;
        self.height = height;
        self.scale = scale;
        self.pixels = PixelBuffer::new(width, height)?;
        self.theme = self.appearance.theme(scale);
        self.rebuild();
        Ok(())
    }
    fn section(&self, key: &str, title: &str) -> Card {
        Card::new(key)
            .padding(18.0 * self.scale)
            .gap(10.0 * self.scale)
            .child(Label::new(format!("{key}-heading"), title).font(Font {
                family: "sans-serif bold".into(),
                size: (17.0 * self.scale).round() as u16,
            }))
    }
    fn settings_page(&self, width: f32) -> Widget {
        let s = self.scale;
        let inner = (width - 36.0 * s).max(0.0);
        Column::new("settings-content")
            .gap(16.0 * s)
            .width(width)
            .child(
                self.section("audio", "Sound")
                    .child(
                        Label::new("audio-help", "Adjust output level and stereo balance.")
                            .typography(TextRole::Caption)
                            .color(self.theme.subtext),
                    )
                    .child(
                        Slider::new("volume", 0.0, 100.0)
                            .label("Volume · steps of 5%")
                            .value(self.settings.volume)
                            .step(5.0)
                            .width(inner),
                    )
                    .child(
                        Slider::new("balance", -1.0, 1.0)
                            .label("Stereo balance · continuous")
                            .value(self.settings.balance)
                            .width(inner),
                    ),
            )
            .child(
                self.section("playback", "Playback")
                    .child(
                        Switch::new("autoplay", "Continue playing automatically")
                            .checked(self.settings.autoplay),
                    )
                    .child(
                        Checkbox::new("normalize", "Normalize volume")
                            .checked(self.settings.normalize),
                    )
                    .child(
                        RadioGroup::new("preset")
                            .accessible_label("Playback preset")
                            .selected(&self.settings.preset)
                            .child(RadioOption::new("balanced", "Balanced"))
                            .child(RadioOption::new("quiet", "Quiet listening"))
                            .child(
                                RadioOption::new(
                                    "unavailable-preset",
                                    "Studio processing unavailable",
                                )
                                .disabled(true),
                            ),
                    ),
            )
            .child(
                self.section("device", "Output device")
                    .child(
                        Label::new("name-label", "Display name")
                            .typography(TextRole::Caption)
                            .color(self.theme.subtext),
                    )
                    .child(
                        TextInput::new("name", &self.settings.name)
                            .accessible_label("Output display name")
                            .width(inner),
                    )
                    .child(
                        Slider::new("locked", 0.0, 100.0)
                            .label("System output locked")
                            .value(30.0)
                            .step(5.0)
                            .disabled(true)
                            .width(inner),
                    ),
            )
            .into()
    }
    fn components_page(&self, width: f32) -> Widget {
        let s = self.scale;
        let inner = (width - 36.0 * s).max(0.0);
        Column::new("gallery-content")
            .width(width)
            .gap(16.0 * s)
            .child(
                self.section("buttons", "Buttons and focus")
                    .child(
                        Row::new("button-row")
                            .gap(10.0 * s)
                            .child(Button::new("normal-button", "Normal"))
                            .child(Button::new("disabled-button", "Disabled").disabled(true)),
                    )
                    .child(
                        Label::new(
                            "focus-hint",
                            "Tab to inspect focus; hover and press to inspect states.",
                        )
                        .typography(TextRole::Caption)
                        .color(self.theme.subtext),
                    ),
            )
            .child(
                self.section("choices", "Boolean controls")
                    .child(Checkbox::new("gallery-check", "Checked checkbox").checked(true))
                    .child(
                        Checkbox::new("gallery-disabled", "Disabled checkbox")
                            .checked(true)
                            .disabled(true),
                    )
                    .child(Switch::new("gallery-switch", "Animated switch").checked(true)),
            )
            .child(
                self.section("validation", "Text and validation")
                    .child(
                        TextInput::new("gallery-error", "Invalid example")
                            .accessible_label("Example invalid field")
                            .invalid(true)
                            .width(inner),
                    )
                    .child(
                        TextInput::new("gallery-readonly", "Read-only example")
                            .accessible_label("Example read-only field")
                            .read_only(true)
                            .width(inner),
                    ),
            )
            .child(
                self.section("ranges", "Range endpoints")
                    .child(
                        Slider::new("minimum", 0.0, 100.0)
                            .label("Minimum")
                            .value(0.0)
                            .width(inner),
                    )
                    .child(
                        Slider::new("maximum", 0.0, 100.0)
                            .label("Maximum")
                            .value(100.0)
                            .width(inner),
                    )
                    .child(
                        Slider::new("gallery-locked", 0.0, 100.0)
                            .label("Disabled")
                            .value(45.0)
                            .disabled(true)
                            .width(inner),
                    ),
            )
            .into()
    }
    fn rebuild(&mut self) {
        let s = self.scale;
        let width = (self.width as f32 - 40.0 * s).max(1.0);
        let content = match self.page {
            Page::Settings => self.settings_page(width),
            Page::Components => self.components_page(width),
        };
        let heading = Row::new("heading")
            .gap(12.0 * s)
            .align(Align::Center)
            .child(
                Label::new("title", "Halley UI")
                    .typography(TextRole::Title)
                    .grow(1.0),
            )
            .child(
                Button::new("theme-light", "Light").style(
                    self.appearance
                        .choice_style(self.appearance == Appearance::Light),
                ),
            )
            .child(
                Button::new("theme-dark", "Dark").style(
                    self.appearance
                        .choice_style(self.appearance == Appearance::Dark),
                ),
            );
        let navigation = Row::new("navigation")
            .gap(10.0 * s)
            .child(
                Button::new("settings-tab", "Settings")
                    .style(self.appearance.choice_style(self.page == Page::Settings)),
            )
            .child(
                Button::new("components-tab", "Components")
                    .style(self.appearance.choice_style(self.page == Page::Components)),
            )
            .child(Label::new("nav-space", "").grow(1.0))
            .child(Button::new("run-checks", "Run checks"));
        self.stack.base_mut().set_content(
            Column::new("app")
                .width(self.width as f32)
                .padding(20.0 * s)
                .gap(12.0 * s)
                .child(heading)
                .child(navigation)
                .child(
                    Scroll::new(match self.page {
                        Page::Settings => "settings-scroll",
                        Page::Components => "components-scroll",
                    })
                    .width(width)
                    .height((self.height as f32 - 270.0 * s).max(80.0 * s))
                    .child(content),
                )
                .child(
                    Row::new("actions")
                        .align(Align::Center)
                        .gap(10.0 * s)
                        .child(Button::new("review", "Review settings"))
                        .child(
                            Label::new("actions-hint", "Changes stay in this demo")
                                .typography(TextRole::Caption)
                                .color(self.theme.subtext),
                        ),
                ),
        );
    }
    fn remember_settings(&mut self) {
        if self.page != Page::Settings {
            return;
        }
        let view = self.stack.base();
        self.settings.volume = view.slider_value("volume").unwrap_or(self.settings.volume);
        self.settings.balance = view
            .slider_value("balance")
            .unwrap_or(self.settings.balance);
        self.settings.autoplay = view.checked("autoplay").unwrap_or(self.settings.autoplay);
        self.settings.normalize = view.checked("normalize").unwrap_or(self.settings.normalize);
        if let Some(value) = view.radio_selected("preset") {
            self.settings.preset = value.into();
        }
        if let Some(value) = view.text_value("name") {
            self.settings.name = value.into();
        }
    }
    pub fn render(&mut self) -> Result<&PixelBuffer> {
        let scene = self
            .stack
            .prepare(
                Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
                &self.theme,
                &mut self.text,
            )?
            .clone();
        self.remember_settings();
        let footer = Column::new("footer")
            .gap(4.0 * self.scale)
            .child(
                Label::new(
                    "values",
                    format!(
                        "Volume {:.0}%   ·   Balance {:+.2}   ·   {}",
                        self.settings.volume, self.settings.balance, self.settings.name
                    ),
                )
                .typography(TextRole::Caption)
                .color(self.theme.subtext),
            )
            .child(
                Label::new("status", &self.status)
                    .typography(TextRole::Caption)
                    .color(self.theme.text),
            )
            .child(
                Label::new(
                    "navigation-help",
                    if scene
                        .scrolls
                        .iter()
                        .any(|s| s.extent.height > s.viewport.size.height)
                    {
                        "Scroll for more · Tab moves focus"
                    } else {
                        "Tab moves focus · Space toggles · Arrow keys adjust"
                    },
                )
                .typography(TextRole::Caption)
                .color(self.theme.subtext),
            );
        self.footer.set_content(footer);
        let footer_scene = self.footer.prepare(
            Rect::new(
                20.0 * self.scale,
                self.height as f32 - 68.0 * self.scale,
                (self.width as f32 - 40.0 * self.scale).max(0.0),
                60.0 * self.scale,
            ),
            &self.theme,
            &mut self.text,
        )?;
        self.pixels
            .surface()
            .clear(self.appearance.palette().background);
        // Background status is painted first; modal backdrops cover it too.
        self.pixels
            .surface()
            .draw(footer_scene, &mut self.text, 1.0);
        self.pixels.surface().draw(&scene, &mut self.text, 1.0);
        self.scene = scene;
        Ok(&self.pixels)
    }
    pub fn input(&mut self, event: InputEvent) -> Result<Vec<LayerEvent>> {
        let events = self.stack.handle_event(event);
        for event in &events {
            if let LayerEvent::View {
                event: UiEvent::Activate(action),
                ..
            } = event
            {
                match action.0.as_str() {
                    "theme-light" | "theme-dark" => {
                        self.appearance = if action.0 == "theme-light" {
                            Appearance::Light
                        } else {
                            Appearance::Dark
                        };
                        self.theme = self.appearance.theme(self.scale);
                        self.rebuild();
                    }
                    "settings-tab" | "components-tab" => {
                        self.remember_settings();
                        self.page = if action.0 == "settings-tab" {
                            Page::Settings
                        } else {
                            Page::Components
                        };
                        self.rebuild();
                    }
                    "run-checks" => self.run_requested = true,
                    "review" => {
                        self.remember_settings();
                        self.stack.open(
                            Dialog::new("review-dialog")
                                .accessible_label("Review settings")
                                .initial_focus("close-review")
                                .child(
                                    Column::new("review-content")
                                        .gap(12.0 * self.scale)
                                        .child(
                                            Label::new("review-title", "Review settings")
                                                .typography(TextRole::Title),
                                        )
                                        .child(Label::new(
                                            "review-value",
                                            format!(
                                                "Volume {:.0}% · {}",
                                                self.settings.volume, self.settings.name
                                            ),
                                        ))
                                        .child(
                                            Checkbox::new("remember", "Remember this choice")
                                                .checked(true),
                                        )
                                        .child(Button::new("close-review", "Close preview")),
                                ),
                        )?;
                    }
                    "close-review" => {
                        self.stack.close_top();
                    }
                    _ => {}
                }
            }
        }
        Ok(events)
    }
    pub fn advance(&mut self, delta: Duration) {
        self.stack.advance_animations(delta);
    }
    pub fn take_run_request(&mut self) -> bool {
        std::mem::take(&mut self.run_requested)
    }
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }
    pub fn save(&mut self, path: &Path) -> Result<()> {
        self.render()?;
        image::save_buffer(
            path,
            self.pixels.pixels(),
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )?;
        Ok(())
    }
    pub fn control_point(&self, key: &str) -> Result<Point> {
        let key = format!("base/{key}");
        let c = self
            .scene
            .controls
            .iter()
            .find(|c| c.key == key)
            .ok_or_else(|| format!("missing control {key}"))?;
        let r = c.rect.intersection(c.clip);
        if r.size.width <= 0.0 || r.size.height <= 0.0 {
            return Err(format!("control {key} is clipped").into());
        }
        Ok(Point::new(
            r.origin.x + r.size.width * 0.5,
            r.origin.y + r.size.height * 0.5,
        ))
    }
    pub fn click(&mut self, key: &str) -> Result<Vec<LayerEvent>> {
        self.render()?;
        let p = self.control_point(key)?;
        let mut out = self.input(InputEvent::PointerDown {
            position: p,
            shift: false,
        })?;
        out.extend(self.input(InputEvent::PointerUp { position: p })?);
        self.render()?;
        Ok(out)
    }
    pub fn key(&mut self, key: Key) -> Result<Vec<LayerEvent>> {
        self.input(InputEvent::KeyDown {
            key,
            modifiers: Default::default(),
        })
    }
    pub fn focus(&mut self, key: &str) -> Result<()> {
        self.stack.focus(&LayerId::Base, key)?;
        self.render()?;
        Ok(())
    }
}
