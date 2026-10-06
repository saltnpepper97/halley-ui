#![cfg(all(feature = "accessibility-unix", target_os = "linux"))]
use halley_ui::accessibility::unix::UnixBridge;
use halley_ui::{Anchor, Button, Column, Rect, TextInput, TextSystem, Theme, UiEvent, UiView};
use std::time::{Duration, Instant};
use zbus::{
    blocking::{Connection, Proxy, connection::Builder},
    zvariant::OwnedObjectPath,
};

struct RegistryProcess(std::process::Child);
impl Drop for RegistryProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
type Object = (String, OwnedObjectPath);
fn children(connection: &Connection, object: &Object) -> Vec<Object> {
    Proxy::new(
        connection,
        object.0.as_str(),
        object.1.as_str(),
        "org.a11y.atspi.Accessible",
    )
    .unwrap()
    .call("GetChildren", &())
    .unwrap()
}
fn name(connection: &Connection, object: &Object) -> String {
    let properties = Proxy::new(
        connection,
        object.0.as_str(),
        object.1.as_str(),
        "org.freedesktop.DBus.Properties",
    )
    .unwrap();
    let value: zbus::zvariant::OwnedValue = properties
        .call("Get", &("org.a11y.atspi.Accessible", "Name"))
        .unwrap();
    String::try_from(value).unwrap()
}
fn find(connection: &Connection, object: &Object, label: &str, depth: usize) -> Option<Object> {
    if name(connection, object) == label {
        return Some(object.clone());
    }
    if depth == 0 {
        return None;
    }
    children(connection, object)
        .into_iter()
        .find_map(|child| find(connection, &child, label, depth - 1))
}
fn prepare(view: &mut UiView, text: &mut TextSystem) -> halley_ui::ui::PreparedView {
    view.prepare(Rect::new(0.0, 0.0, 400.0, 180.0), &Theme::default(), text)
        .unwrap()
        .clone()
}
fn drain(bridge: &mut UnixBridge, view: &mut UiView) -> Vec<UiEvent> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let actions = bridge.actions();
        if !actions.is_empty() {
            return actions
                .into_iter()
                .flat_map(|action| view.handle_accessibility(action))
                .collect();
        }
        assert!(
            Instant::now() < deadline,
            "AT-SPI request did not reach the host"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
#[ignore = "run only inside dbus-run-session with HALLEY_UI_PRIVATE_ATSPI_TEST=1"]
fn linux_bridge_exposes_controls_and_round_trips_real_atspi_requests() {
    assert_eq!(
        std::env::var("HALLEY_UI_PRIVATE_ATSPI_TEST").as_deref(),
        Ok("1"),
        "This test must use a separate session bus"
    );
    assert_eq!(
        std::env::var("GSETTINGS_BACKEND").as_deref(),
        Ok("memory"),
        "Use memory-only settings for this private-bus test"
    );
    let _launcher = RegistryProcess(
        std::process::Command::new("/usr/lib/at-spi-bus-launcher")
            .env("GSETTINGS_BACKEND", "memory")
            .env("DISPLAY", "")
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let session = Connection::session().unwrap();
    let dbus = zbus::blocking::fdo::DBusProxy::new(&session).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !dbus
        .name_has_owner("org.a11y.Bus".try_into().unwrap())
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }

    let bus = Proxy::new(&session, "org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus").unwrap();
    let address: String = bus.call("GetAddress", &()).unwrap();
    let status = Proxy::new(&session, "org.a11y.Bus", "/org/a11y/bus", "org.a11y.Status").unwrap();
    status.set_property("IsEnabled", true).unwrap();
    let atspi = Builder::address(address.as_str()).unwrap().build().unwrap();
    let _registry_process = RegistryProcess(
        std::process::Command::new("/usr/lib/at-spi2-registryd")
            .env("AT_SPI_BUS_ADDRESS", &address)
            .env("DISPLAY", "")
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let dbus = zbus::blocking::fdo::DBusProxy::new(&atspi).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !dbus
        .name_has_owner("org.a11y.atspi.Registry".try_into().unwrap())
        .unwrap()
    {
        assert!(Instant::now() < deadline, "private registry did not start");
        std::thread::sleep(Duration::from_millis(20));
    }
    let registry: (String, OwnedObjectPath) = (
        "org.a11y.atspi.Registry".into(),
        "/org/a11y/atspi/accessible/root".try_into().unwrap(),
    );
    let mut view = UiView::new("test").anchor(Anchor::TopLeft).content(
        Column::new("form")
            .child(TextInput::new("name", "Work").accessible_label("Cluster name"))
            .child(Button::new("create", "Create")),
    );
    let mut text = TextSystem::new();
    let scene = prepare(&mut view, &mut text);
    let mut bridge = UnixBridge::new(&scene, "Halley UI bridge test", || {});
    bridge.update(&scene, "Halley UI bridge test", true);
    let deadline = Instant::now() + Duration::from_secs(5);
    let input = loop {
        if let Some(input) = find(&atspi, &registry, "Cluster name", 5) {
            break input;
        }
        assert!(
            Instant::now() < deadline,
            "UI tree did not appear on the private accessibility bus"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let editable = Proxy::new(
        &atspi,
        input.0.as_str(),
        input.1.as_str(),
        "org.a11y.atspi.EditableText",
    )
    .unwrap();
    let accepted: bool = editable.call("SetTextContents", &("Research",)).unwrap();
    assert!(accepted);
    assert!(
        drain(&mut bridge, &mut view).contains(&UiEvent::TextChanged {
            key: "name".into(),
            value: "Research".into()
        })
    );
    let scene = prepare(&mut view, &mut text);
    bridge.update(&scene, "Halley UI bridge test", true);
    let text_proxy = Proxy::new(
        &atspi,
        input.0.as_str(),
        input.1.as_str(),
        "org.a11y.atspi.Text",
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let actual: String = text_proxy.call("GetText", &(0i32, -1i32)).unwrap();
        if actual == "Research" {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    let create = find(&atspi, &registry, "Create", 5).unwrap();
    let action = Proxy::new(
        &atspi,
        create.0.as_str(),
        create.1.as_str(),
        "org.a11y.atspi.Action",
    )
    .unwrap();
    let accepted: bool = action.call("DoAction", &(0i32,)).unwrap();
    assert!(accepted);
    assert_eq!(
        drain(&mut bridge, &mut view),
        vec![UiEvent::Activate(halley_ui::ActionId::new("create"))]
    );
    let component = Proxy::new(
        &atspi,
        input.0.as_str(),
        input.1.as_str(),
        "org.a11y.atspi.Component",
    )
    .unwrap();
    let accepted: bool = component.call("GrabFocus", &()).unwrap();
    assert!(accepted);
    assert!(drain(&mut bridge, &mut view).contains(&UiEvent::FocusChanged(Some("name".into()))));
}
