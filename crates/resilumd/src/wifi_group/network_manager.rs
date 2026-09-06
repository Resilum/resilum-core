use std::time::Duration;

use dbus::Path;
use dbus::blocking::Connection;
use resilum_core::WifiGroup;

use super::{Lowering, Raised, RaisesAGroup, settings};

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const SETTINGS: &str = "org.freedesktop.NetworkManager.Settings";
const SETTINGS_PATH: &str = "/org/freedesktop/NetworkManager/Settings";
const A_STORED_CONNECTION: &str = "org.freedesktop.NetworkManager.Settings.Connection";
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
    if let Some(ours) = settings::id_of(&connection) {
        for left_behind in every_profile_left_under(&bus, &ours) {
            forget(&bus, &left_behind);
        }
    }
    let manager = bus.with_proxy(NM, NM_PATH, ANSWERS_WITHIN);
    let (device,): (Path,) = manager
        .method_call(NM, "GetDeviceByIpIface", (interface,))
        .map_err(|e| format!("no device named {interface}: {e}"))?;
    let (stored, active): (Path, Path) = manager
        .method_call(
            NM,
            "AddAndActivateConnection",
            (connection, device, Path::new("/").unwrap_or_default()),
        )
        .map_err(|e| format!("NetworkManager would not {what_for}: {e}"))?;
    let stored = stored.into_static();
    let active = active.into_static();
    Ok(Box::new(move || {
        deactivate(&bus, &active);
        forget(&bus, &stored);
    }))
}

fn every_profile_left_under(bus: &Connection, id: &str) -> Vec<Path<'static>> {
    let settings = bus.with_proxy(NM, SETTINGS_PATH, ANSWERS_WITHIN);
    let listed: Result<(Vec<Path>,), _> = settings.method_call(SETTINGS, "ListConnections", ());
    let Ok((paths,)) = listed else {
        return Vec::new();
    };
    paths
        .into_iter()
        .map(Path::into_static)
        .filter(|path| named(bus, path).as_deref() == Some(id))
        .collect()
}

fn named(bus: &Connection, stored: &Path<'static>) -> Option<String> {
    let connection = bus.with_proxy(NM, stored.clone(), ANSWERS_WITHIN);
    let read: Result<(settings::Connection,), _> =
        connection.method_call(A_STORED_CONNECTION, "GetSettings", ());
    settings::id_of(&read.ok()?.0)
}

fn forget(bus: &Connection, stored: &Path<'static>) {
    let connection = bus.with_proxy(NM, stored.clone(), ANSWERS_WITHIN);
    let deleted: Result<(), _> = connection.method_call(A_STORED_CONNECTION, "Delete", ());
    if let Err(error) = deleted {
        tracing::warn!(%error, "a wi-fi group profile was left in NetworkManager");
    }
}

fn deactivate(bus: &Connection, active: &Path<'static>) {
    let manager = bus.with_proxy(NM, NM_PATH, ANSWERS_WITHIN);
    let taken_down: Result<(), _> =
        manager.method_call(NM, "DeactivateConnection", (active.clone(),));
    if let Err(error) = taken_down {
        tracing::warn!(%error, "the wi-fi group would not come down");
    }
}
