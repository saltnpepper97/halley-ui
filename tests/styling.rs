use halley_ui::{
    Anchor, Button, Color, Column, Font, InputEvent, Insets, Key, Label, List, ListItem, Modifiers,
    Outline, Point, Rect, Row, Size, Style, StyleSheet, TextInput, TextRole, TextSystem, Theme,
    UiView,
    ui::{CardStyle, LayoutError, PaintItem, PreparedView},
};
use std::cell::Cell;

const RED: Color = Color::rgb8(220, 40, 50);
const GREEN: Color = Color::rgb8(30, 180, 90);
const BLUE: Color = Color::rgb8(40, 80, 210);
const GOLD: Color = Color::rgb8(220, 170, 30);
fn metrics(font: &Font, value: &str) -> Size {
    Size::new(
        value.chars().count() as f32 * f32::from(font.size),
        f32::from(font.size),
    )
}
fn prepare(view: &mut UiView, theme: &Theme) -> PreparedView {
    view.prepare_with_measure(Rect::new(0.0, 0.0, 800.0, 600.0), theme, metrics)
        .unwrap()
        .clone()
}
fn surface(scene: &PreparedView, key: &str) -> CardStyle {
    scene
        .items
        .iter()
        .find_map(|item| match item {
            PaintItem::Card { key: k, style, .. } if k == key => Some(*style),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing surface {key}"))
}
fn text(scene: &PreparedView, key: &str) -> (Font, Color, Rect) {
    scene
        .items
        .iter()
        .find_map(|item| match item {
            PaintItem::Text {
                key: k,
                font,
                color,
                rect,
                ..
            } if k == key => Some((font.clone(), *color, *rect)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing text {key}"))
}
fn point(scene: &PreparedView, key: &str) -> Point {
    let r = scene.rects[key];
    Point::new(r.origin.x + 3.0, r.origin.y + 3.0)
}
fn key(view: &mut UiView, key: Key, control: bool) {
    view.handle_event(InputEvent::KeyDown {
        key,
        modifiers: Modifiers {
            control,
            ..Default::default()
        },
    });
}

#[test]
fn partial_widget_overrides_keep_theme_states_and_metrics_and_explicit_padding_wins() {
    let mut theme = Theme::default();
    theme.styles.button.font = Some(Font {
        family: "monospace".into(),
        size: 20,
    });
    theme.styles.button.padding = Some(Insets::xy(20.0, 10.0));
    theme.styles.button.normal.border = Some(GOLD);
    theme.styles.button.normal.border_width = Some(3.0);
    theme.styles.button.hovered.fill = Some(BLUE);
    let local = StyleSheet {
        normal: Style {
            fill: Some(GREEN),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut view = UiView::new("partial").anchor(Anchor::TopLeft).content(
        Row::new("row")
            .align(halley_ui::ui::Align::Start)
            .child(Button::new("inherit", "Hi").style(local.clone()))
            .child(Button::new("zero", "Hi").style(local).padding(0.0)),
    );
    let scene = prepare(&mut view, &theme);
    assert_eq!(scene.rects["inherit"].size, Size::new(80.0, 40.0));
    assert_eq!(scene.rects["zero"].size, Size::new(40.0, 20.0));
    assert_eq!(surface(&scene, "partial/inherit").fill, GREEN);
    assert_eq!(surface(&scene, "partial/inherit").border, GOLD);
    assert_eq!(text(&scene, "partial/inherit/label").0.size, 20);
    view.handle_event(InputEvent::PointerMove {
        position: point(&scene, "inherit"),
    });
    let hovered = prepare(&mut view, &theme);
    assert_eq!(surface(&hovered, "partial/inherit").fill, BLUE);
    assert_eq!(surface(&hovered, "partial/inherit").border, GOLD);
    assert_eq!(hovered.rects, scene.rects);
}

#[test]
fn hovered_pressed_focused_and_error_rules_merge_without_moving_hit_geometry() {
    let mut theme = Theme::default();
    theme.styles.button.hovered.fill = Some(BLUE);
    theme.styles.button.pressed.fill = Some(RED);
    theme.styles.button.focused.outline = Some(Outline {
        color: GOLD,
        width: 4.0,
    });
    theme.styles.button.error.border = Some(GREEN);
    theme.styles.button.error.border_width = Some(3.0);
    let mut view = UiView::new("states")
        .anchor(Anchor::TopLeft)
        .content(Button::new("go", "Go").invalid(true));
    let initial = prepare(&mut view, &theme);
    let p = point(&initial, "go");
    view.handle_event(InputEvent::PointerMove { position: p });
    view.handle_event(InputEvent::PointerDown {
        position: p,
        shift: false,
    });
    let pressed = prepare(&mut view, &theme);
    let style = surface(&pressed, "states/go");
    assert_eq!(style.fill, RED);
    assert_eq!(style.border, GREEN);
    assert_eq!(style.border_width, 3.0);
    assert_eq!(surface(&pressed, "states/go/focus").border, GOLD);
    assert_eq!(surface(&pressed, "states/go/focus").border_width, 4.0);
    assert_eq!(pressed.controls[0].rect, initial.controls[0].rect);
    assert_eq!(pressed.hits[0].rect, initial.hits[0].rect);
    view.handle_event(InputEvent::PointerCancel);
    assert_eq!(surface(&prepare(&mut view, &theme), "states/go").fill, BLUE);
    view.clear_focus();
    view.handle_event(InputEvent::PointerMove {
        position: Point::new(-1.0, -1.0),
    });
    assert_eq!(
        surface(&prepare(&mut view, &theme), "states/go").fill,
        theme.button.fill
    );
}

#[test]
fn selected_list_rules_coexist_with_hover_focus_and_item_overrides() {
    let mut theme = Theme::default();
    theme.styles.list_item.selected.fill = Some(RED);
    theme.styles.list_item.selected.border = Some(GOLD);
    theme.styles.list_item.selected.border_width = Some(1.0);
    theme.styles.list_item.hovered.fill = Some(BLUE);
    let owner = StyleSheet {
        selected: Style {
            text: Some(GREEN),
            ..Default::default()
        },
        ..Default::default()
    };
    let item = StyleSheet {
        focused: Style {
            outline: Some(Outline {
                color: GREEN,
                width: 3.0,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut view = UiView::new("list").anchor(Anchor::TopLeft).content(
        List::new("results")
            .width(220.0)
            .height(90.0)
            .selected("one")
            .style(StyleSheet {
                normal: Style {
                    fill: Some(GREEN),
                    radius: Some(12.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .item_style(owner)
            .child(ListItem::new("one", "One").style(item).height(40.0))
            .child(ListItem::new("two", "Two").height(40.0)),
    );
    let scene = prepare(&mut view, &theme);
    assert_eq!(view.focused(), None);
    assert_eq!(surface(&scene, "list/results").fill, GREEN);
    assert_eq!(surface(&scene, "list/results").radius, 12.0);
    assert_eq!(surface(&scene, "list/one").fill, RED);
    assert_eq!(text(&scene, "list/one/label").1, GREEN);
    view.focus("results").unwrap();
    view.handle_event(InputEvent::PointerMove {
        position: point(&scene, "one"),
    });
    let scene = prepare(&mut view, &theme);
    assert_eq!(surface(&scene, "list/one").fill, BLUE);
    assert_eq!(surface(&scene, "list/one").border, GOLD);
    assert_eq!(surface(&scene, "list/one/focus").border, GREEN);
    assert_eq!(
        surface(&scene, "list/results/focus").border,
        theme.card.border
    );
    assert_eq!(view.selected("results"), Some("one"));
}

#[test]
fn disabled_rules_win_without_hover_capture_or_repeated_alpha_loss() {
    let mut theme = Theme::default();
    theme.styles.button.hovered.fill = Some(RED);
    theme.styles.button.pressed.fill = Some(RED);
    theme.styles.button.disabled.fill = Some(GREEN);
    theme.styles.button.disabled.text = Some(GOLD);
    let mut view = UiView::new("disabled").content(Button::new("go", "Go").disabled(true));
    let scene = prepare(&mut view, &theme);
    let p = point(&scene, "go");
    view.handle_event(InputEvent::PointerMove { position: p });
    view.handle_event(InputEvent::PointerDown {
        position: p,
        shift: false,
    });
    assert!(
        view.handle_event(InputEvent::PointerUp { position: p })
            .is_empty()
    );
    for _ in 0..3 {
        view.invalidate();
        let scene = prepare(&mut view, &theme);
        assert_eq!(surface(&scene, "disabled/go").fill, GREEN);
        assert_eq!(text(&scene, "disabled/go/label").1, GOLD);
        assert_eq!(view.focused(), None);
    }
}

#[test]
fn text_input_styles_control_placeholder_selection_caret_preedit_and_outline_suppression() {
    let mut theme = Theme::default();
    theme.styles.text_input.normal.placeholder = Some(GOLD);
    theme.styles.text_input.normal.selection = Some(GREEN);
    theme.styles.text_input.normal.caret = Some(RED);
    theme.styles.text_input.normal.preedit = Some(BLUE);
    theme.styles.text_input.focused.border_width = Some(5.0);
    let mut view = UiView::new("editor").content(TextInput::new("name", "").placeholder("Name"));
    let scene = prepare(&mut view, &theme);
    assert_eq!(text(&scene, "editor/name/text").1, GOLD);
    view.focus("name").unwrap();
    view.handle_event(InputEvent::Text("abcd".into()));
    key(&mut view, Key::A, true);
    let scene = prepare(&mut view, &theme);
    assert_eq!(surface(&scene, "editor/name").border_width, 5.0);
    assert_eq!(surface(&scene, "editor/name/caret").fill, RED);
    assert!(
        scene
            .items
            .iter()
            .any(|item| matches!(item, PaintItem::Card { key, style, .. }
        if key.contains("/selection/") && style.fill == GREEN))
    );
    view.handle_event(InputEvent::Preedit {
        text: "日本".into(),
        cursor: Some(3),
    });
    assert_eq!(
        surface(&prepare(&mut view, &theme), "editor/name/preedit").fill,
        BLUE
    );
    let mut button = UiView::new("no-ring").content(Button::new("go", "Go").style(StyleSheet {
        focused: Style {
            outline: Some(Outline {
                color: GOLD,
                width: 0.0,
            }),
            ..Default::default()
        },
        ..Default::default()
    }));
    prepare(&mut button, &theme);
    button.focus("go").unwrap();
    assert!(
        !prepare(&mut button, &theme)
            .items
            .iter()
            .any(|p| matches!(p, PaintItem::Card { key, .. } if key.ends_with("/focus")))
    );
}

#[test]
fn custom_content_inherits_state_text_and_font_and_explicit_labels_keep_overrides() {
    let mut theme = Theme::default();
    let large = Font {
        family: "monospace".into(),
        size: 22,
    };
    theme.styles.button.font = Some(large.clone());
    theme.styles.button.normal.text = Some(GREEN);
    theme.styles.button.hovered.text = Some(BLUE);
    let mut view =
        UiView::new("custom")
            .anchor(Anchor::TopLeft)
            .content(Button::new("go", "").child(
                Row::new("row").child(Label::new("inherited", "Go")).child(
                    Label::new("explicit", "!").color(RED).font(Font {
                        family: "monospace".into(),
                        size: 10,
                    }),
                ),
            ));
    let normal = prepare(&mut view, &theme);
    assert_eq!(text(&normal, "custom/inherited").0, large);
    assert_eq!(text(&normal, "custom/inherited").1, GREEN);
    assert_eq!(normal.rects["inherited"].size.width, 44.0);
    view.handle_event(InputEvent::PointerMove {
        position: point(&normal, "go"),
    });
    let hovered = prepare(&mut view, &theme);
    assert_eq!(text(&hovered, "custom/inherited").1, BLUE);
    assert_eq!(text(&hovered, "custom/explicit").1, RED);
    assert_eq!(text(&hovered, "custom/explicit").0.size, 10);
    assert_eq!(hovered.rects, normal.rects);
}

#[test]
fn theme_changes_invalidate_cached_fonts_spacing_and_colors_and_keep_retained_state() {
    let mut theme = Theme::default();
    let mut view = UiView::new("cache").anchor(Anchor::TopLeft).content(
        Column::new("column")
            .child(TextInput::new("name", "Initial"))
            .child(Button::new("go", "Go"))
            .child(
                List::new("results")
                    .selected("one")
                    .child(ListItem::new("one", "One")),
            ),
    );
    let calls = Cell::new(0);
    let mut measure = |font: &Font, value: &str| {
        calls.set(calls.get() + 1);
        metrics(font, value)
    };
    let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);
    view.prepare_with_measure(bounds, &theme, &mut measure)
        .unwrap();
    let first = calls.get();
    view.prepare_with_measure(bounds, &theme, &mut measure)
        .unwrap();
    assert_eq!(calls.get(), first);
    view.set_text_value("name", "Retained");
    view.focus("name").unwrap();
    let before = prepare(&mut view, &theme);
    theme.styles.button.font = Some(Font {
        family: "monospace".into(),
        size: 28,
    });
    theme.styles.button.normal.fill = Some(BLUE);
    theme.spacing.button_padding = Insets::xy(25.0, 14.0);
    theme.spacing.container_gap = 6.0;
    let after = view
        .prepare_with_measure(bounds, &theme, &mut measure)
        .unwrap()
        .clone();
    assert!(calls.get() > first);
    assert!(after.rects["go"].size.height > before.rects["go"].size.height);
    assert_eq!(surface(&after, "cache/go").fill, BLUE);
    assert_eq!(text(&after, "cache/go/label").0.size, 28);
    assert_eq!(view.text_value("name"), Some("Retained"));
    assert_eq!(view.focused(), Some("name"));
    assert_eq!(view.selected("results"), Some("one"));
}

#[test]
fn typography_roles_and_explicit_fonts_use_the_same_measurement_and_painting() {
    let mut theme = Theme::default();
    theme.typography.title = Some(Font {
        family: "monospace".into(),
        size: 30,
    });
    theme.typography.caption = Some(Font {
        family: "monospace".into(),
        size: 9,
    });
    let mut view = UiView::new("type").anchor(Anchor::TopLeft).content(
        Column::new("column")
            .child(Label::new("title", "Hello").typography(TextRole::Title))
            .child(Label::new("caption", "Small").typography(TextRole::Caption))
            .child(
                Label::new("explicit", "Fixed")
                    .typography(TextRole::Title)
                    .font(Font {
                        family: "monospace".into(),
                        size: 13,
                    }),
            ),
    );
    let scene = prepare(&mut view, &theme);
    assert_eq!(text(&scene, "type/title").0.size, 30);
    assert_eq!(text(&scene, "type/caption").0.size, 9);
    assert_eq!(text(&scene, "type/explicit").0.size, 13);
    assert_eq!(scene.rects["title"].size.height, 30.0);
    assert_eq!(scene.rects["caption"].size.height, 9.0);
    theme.typography.title.as_mut().unwrap().size = 40;
    assert_eq!(prepare(&mut view, &theme).rects["title"].size.height, 40.0);
}

#[test]
fn themed_input_font_updates_caret_hits_and_ime_geometry_without_losing_selection() {
    let mut theme = Theme::default();
    let mut service = TextSystem::new();
    let bounds = Rect::new(20.0, 30.0, 400.0, 160.0);
    let mut view = UiView::new("ime")
        .anchor(Anchor::TopLeft)
        .content(TextInput::new("name", "abcdef").width(300.0));
    view.prepare(bounds, &theme, &mut service).unwrap();
    view.focus("name").unwrap();
    key(&mut view, Key::End, false);
    let before = view.prepare(bounds, &theme, &mut service).unwrap().controls[0].clone();
    let selected = view.text_selection("name");
    theme.styles.text_input.font = Some(Font {
        family: "monospace".into(),
        size: 26,
    });
    theme.styles.text_input.padding = Some(Insets::xy(18.0, 12.0));
    let scene = view.prepare(bounds, &theme, &mut service).unwrap().clone();
    let control = &scene.controls[0];
    assert!(control.caret.origin.x > before.caret.origin.x);
    assert_eq!(control.content.origin.x, control.rect.origin.x + 18.0);
    assert_eq!(view.text_selection("name"), selected);
    assert_eq!(text(&scene, "ime/name/text").0.size, 26);
    let moved = scene.translated(Point::new(35.0, -8.0));
    let ime = view.ime_state_in(&moved);
    assert!(
        matches!(ime, halley_ui::UiEvent::Ime { caret, .. } if caret == moved.controls[0].caret.intersection(moved.controls[0].clip))
    );
    let p = Point::new(control.text_origin.x, control.text_origin.y + 3.0);
    view.handle_event(InputEvent::PointerDown {
        position: p,
        shift: false,
    });
    assert_eq!(view.text_selection("name"), Some((0, 0)));
}

#[test]
fn invalid_style_values_are_rejected_even_in_inactive_state_rules() {
    let mut theme = Theme::default();
    theme.styles.button.pressed.border_width = Some(f32::NAN);
    let mut view = UiView::new("invalid").content(Button::new("go", "Go"));
    assert!(matches!(
        view.prepare_with_measure(Rect::new(0.0, 0.0, 400.0, 200.0), &theme, metrics),
        Err(LayoutError::InvalidStyle(_))
    ));
    theme = Theme::default();
    view.set_content(Button::new("go", "Go").style(StyleSheet {
        padding: Some(Insets::all(-1.0)),
        ..Default::default()
    }));
    assert!(matches!(
        view.prepare_with_measure(Rect::new(0.0, 0.0, 400.0, 200.0), &theme, metrics),
        Err(LayoutError::InvalidStyle(_))
    ));
}

#[test]
fn component_fills_reach_software_pixels_and_change_with_hover() {
    use halley_ui::software::PixelBuffer;
    let mut theme = Theme::default();
    theme.styles.button.normal = Style {
        fill: Some(RED),
        radius: Some(0.0),
        border_width: Some(0.0),
        ..Default::default()
    };
    theme.styles.button.hovered.fill = Some(BLUE);
    theme.styles.list.normal = Style {
        fill: Some(GREEN),
        radius: Some(0.0),
        ..Default::default()
    };
    let mut view = UiView::new("pixels").anchor(Anchor::TopLeft).content(
        Row::new("row")
            .child(Button::new("go", "").width(40.0).height(30.0).padding(0.0))
            .child(List::new("results").width(40.0).height(30.0)),
    );
    let mut service = TextSystem::new();
    let render = |scene: &PreparedView, service: &mut TextSystem| {
        let mut pixels = PixelBuffer::new(80, 30).unwrap();
        pixels.surface().draw(scene, service, 1.0);
        let sample = |x: usize| {
            let i = (10 * 80 + x) * 4;
            pixels.pixels()[i..i + 4].to_vec()
        };
        (sample(10), sample(50))
    };
    let scene = prepare(&mut view, &theme);
    assert_eq!(
        render(&scene, &mut service),
        (vec![220, 40, 50, 255], vec![30, 180, 90, 255])
    );
    view.handle_event(InputEvent::PointerMove {
        position: point(&scene, "go"),
    });
    let scene = prepare(&mut view, &theme);
    assert_eq!(
        render(&scene, &mut service),
        (vec![40, 80, 210, 255], vec![30, 180, 90, 255])
    );
}
