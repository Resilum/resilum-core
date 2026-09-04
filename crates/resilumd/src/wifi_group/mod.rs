mod address;
mod dhcp;
mod hosting;
mod iwd;
mod joining;
mod network_manager;
pub mod radio_interface;
mod settings;
mod supplicant;

use std::time::Duration;

use resilum_core::WifiGroup;

pub use hosting::WhetherWeHostTheGroup;
pub use joining::WhetherWeJoinTheGroup;

pub type Lowering = Box<dyn FnOnce() + Send>;

pub struct Raised {
    pub carried_on: String,
    pub already_addressed: bool,
    pub lower: Lowering,
}

pub trait RaisesAGroup: Send + Sync {
    fn raise(&self, group: &WifiGroup, interface: &str) -> Result<Raised, String>;
}

fn whoever_holds_the_radio() -> Option<Box<dyn RaisesAGroup>> {
    if let Some(wpa) = supplicant::if_it_can_own_a_p2p_group() {
        return Some(Box::new(wpa));
    }
    if let Some(nm) = network_manager::if_it_is_running() {
        return Some(Box::new(nm));
    }
    iwd::if_it_is_running().map(|iwd| Box::new(iwd) as Box<dyn RaisesAGroup>)
}

fn owns_a_name_on_the_bus(name: &str) -> bool {
    let Ok(bus) = dbus::blocking::Connection::new_system() else {
        return false;
    };
    let dbus = bus.with_proxy(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        Duration::from_secs(5),
    );
    let owned: Result<(bool,), _> =
        dbus.method_call("org.freedesktop.DBus", "NameHasOwner", (name,));
    owned.is_ok_and(|(owned,)| owned)
}
