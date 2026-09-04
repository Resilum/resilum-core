use std::time::Duration;

use dbus::Path;
use dbus::blocking::Connection;
use resilum_core::WifiGroup;

use super::{Lowering, Raised, RaisesAGroup, settings};

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const ANSWERS_WITHIN: Duration = Duration::from_secs(25);

pub struct NetworkManager;

pub fn if_it_is_running() -> Option<NetworkManager> {
    super::owns_a_name_on_the_bus(NM).then_some(NetworkManager)
}

impl RaisesAGroup for NetworkManager {
    fn raise(&self, group: &WifiGroup, interface: &str) -> Result<Raised, String> {
        Ok(Raised {
            carried_on: interface.to_owned(),
            already_addressed: true,
            lower: activate(
                settings::a_group_owned_by_us(group, interface),
                interface,
                "raise the group",
            )?,
        })
    }
}

pub(super) fn activate(
    connection: settings::Connection,
    interface: &str,
    what_for: &str,
) -> Result<Lowering, String> {
    let bus = Connection::new_system().map_err(|e| format!("no system bus: {e}"))?;
    let manager = bus.with_proxy(NM, NM_PATH, ANSWERS_WITHIN);
    let (device,): (Path,) = manager
        .method_call(NM, "GetDeviceByIpIface", (interface,))
        .map_err(|e| format!("no device named {interface}: {e}"))?;
    let (_, active): (Path, Path) = manager
        .method_call(
            NM,
            "AddAndActivateConnection",
            (connection, device, Path::new("/").unwrap_or_default()),
        )
        .map_err(|e| format!("NetworkManager would not {what_for}: {e}"))?;
    let active = active.into_static();
    Ok(Box::new(move || deactivate(&bus, &active)))
}

fn deactivate(bus: &Connection, active: &Path<'static>) {
    let manager = bus.with_proxy(NM, NM_PATH, ANSWERS_WITHIN);
    let taken_down: Result<(), _> =
        manager.method_call(NM, "DeactivateConnection", (active.clone(),));
    if let Err(error) = taken_down {
        tracing::warn!(%error, "the wi-fi group would not come down");
    }
}
