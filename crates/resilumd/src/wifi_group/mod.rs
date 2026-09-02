mod address;
mod dhcp;
mod hosting;
mod iwd;
mod network_manager;
mod radio_interface;
mod settings;
mod supplicant;

use std::time::Duration;

use resilum_core::WifiGroup;

pub use hosting::WhetherWeHostTheGroup;

pub type Lowering = Box<dyn FnOnce() + Send>;

pub trait RaisesAGroup: Send + Sync {
    fn raise(&self, group: &WifiGroup, interface: &str) -> Result<Lowering, String>;

    fn addresses_the_interface_itself(&self) -> bool;
}

fn whoever_holds_the_radio() -> Option<Box<dyn RaisesAGroup>> {
    if let Some(nm) = network_manager::if_it_is_running() {
        return Some(Box::new(nm));
    }
    if let Some(iwd) = iwd::if_it_is_running() {
        return Some(Box::new(iwd));
    }
    supplicant::if_it_is_running().map(|wpa| Box::new(wpa) as Box<dyn RaisesAGroup>)
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
