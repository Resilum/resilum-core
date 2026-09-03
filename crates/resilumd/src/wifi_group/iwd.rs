use std::collections::HashMap;
use std::time::Duration;

use dbus::Path;
use dbus::arg::{RefArg, Variant};
use dbus::blocking::Connection;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use resilum_core::WifiGroup;

use super::{Raised, RaisesAGroup};

const IWD: &str = "net.connman.iwd";
const DEVICE: &str = "net.connman.iwd.Device";
const ACCESS_POINT: &str = "net.connman.iwd.AccessPoint";
const ANSWERS_WITHIN: Duration = Duration::from_secs(25);
const STATION: &str = "station";
const AP: &str = "ap";

type Objects = HashMap<Path<'static>, HashMap<String, HashMap<String, Variant<Box<dyn RefArg>>>>>;

pub struct Iwd;

pub fn if_it_is_running() -> Option<Iwd> {
    super::owns_a_name_on_the_bus(IWD).then_some(Iwd)
}

impl RaisesAGroup for Iwd {
    fn raise(&self, group: &WifiGroup, interface: &str) -> Result<Raised, String> {
        let bus = Connection::new_system().map_err(|e| format!("no system bus: {e}"))?;
        let device = the_device_named(&bus, interface)?;
        mode(&bus, &device, AP)?;
        started(&bus, &device, group)?;
        let lowering = device.clone();
        Ok(Raised {
            carried_on: interface.to_owned(),
            already_addressed: false,
            lower: Box::new(move || stop(&lowering)),
        })
    }
}

fn started(bus: &Connection, device: &Path<'static>, group: &WifiGroup) -> Result<(), String> {
    let mut refused = String::new();
    for _ in 0..10 {
        let proxy = bus.with_proxy(IWD, device.clone(), ANSWERS_WITHIN);
        match proxy.method_call(
            ACCESS_POINT,
            "Start",
            (group.ssid.as_str(), group.passphrase.as_str()),
        ) {
            Ok(()) => return Ok(()),
            Err(e) => refused = e.to_string(),
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    let _ = mode(bus, device, STATION);
    Err(format!("iwd would not start the group: {refused}"))
}

fn stop(device: &Path<'static>) {
    let Ok(bus) = Connection::new_system() else {
        return;
    };
    let proxy = bus.with_proxy(IWD, device.clone(), ANSWERS_WITHIN);
    let stopped: Result<(), _> = proxy.method_call(ACCESS_POINT, "Stop", ());
    if let Err(error) = stopped {
        tracing::warn!(%error, "iwd would not stop the group");
    }
    if let Err(error) = mode(&bus, device, STATION) {
        tracing::warn!(%error, "the radio was left out of station mode");
    }
}

fn mode(bus: &Connection, device: &Path<'static>, wanted: &str) -> Result<(), String> {
    bus.with_proxy(IWD, device.clone(), ANSWERS_WITHIN)
        .set(DEVICE, "Mode", Variant(wanted.to_owned()))
        .map_err(|e| format!("iwd would not put the radio in {wanted} mode: {e}"))
}

fn the_device_named(bus: &Connection, interface: &str) -> Result<Path<'static>, String> {
    let root = bus.with_proxy(IWD, "/", ANSWERS_WITHIN);
    let (objects,): (Objects,) = root
        .method_call(
            "org.freedesktop.DBus.ObjectManager",
            "GetManagedObjects",
            (),
        )
        .map_err(|e| format!("iwd named no objects: {e}"))?;
    objects
        .into_iter()
        .find(|(_, interfaces)| named(interfaces.get(DEVICE)) == Some(interface))
        .map(|(path, _)| path)
        .ok_or_else(|| format!("iwd holds no device named {interface}"))
}

fn named(device: Option<&HashMap<String, Variant<Box<dyn RefArg>>>>) -> Option<&str> {
    device?.get("Name")?.0.as_str()
}
