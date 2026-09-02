use std::collections::HashMap;

use dbus::arg::{RefArg, Variant};
use resilum_core::WifiGroup;

pub(super) type Section = HashMap<String, Variant<Box<dyn RefArg>>>;
pub(super) type Connection = HashMap<String, Section>;

fn text(value: &str) -> Variant<Box<dyn RefArg>> {
    Variant(Box::new(String::from(value)))
}

pub(super) fn a_group_owned_by_us(group: &WifiGroup, interface: &str) -> Connection {
    HashMap::from([
        (String::from("connection"), which_connection(interface)),
        (String::from("802-11-wireless"), an_access_point(group)),
        (
            String::from("802-11-wireless-security"),
            locked_with(&group.passphrase),
        ),
        (String::from("ipv4"), at_the_owner_address(group)),
        (
            String::from("ipv6"),
            Section::from([(String::from("method"), text("ignore"))]),
        ),
    ])
}

fn which_connection(interface: &str) -> Section {
    Section::from([
        (String::from("id"), text("resilum-group")),
        (String::from("type"), text("802-11-wireless")),
        (String::from("interface-name"), text(interface)),
        (
            String::from("autoconnect"),
            Variant(Box::new(false) as Box<dyn RefArg>),
        ),
    ])
}

fn an_access_point(group: &WifiGroup) -> Section {
    Section::from([
        (
            String::from("ssid"),
            Variant(Box::new(group.ssid.as_bytes().to_vec()) as Box<dyn RefArg>),
        ),
        (String::from("mode"), text("ap")),
        (String::from("band"), text("bg")),
    ])
}

fn locked_with(passphrase: &str) -> Section {
    Section::from([
        (String::from("key-mgmt"), text("wpa-psk")),
        (String::from("psk"), text(passphrase)),
        (
            String::from("proto"),
            Variant(Box::new(vec![String::from("rsn")]) as Box<dyn RefArg>),
        ),
        (
            String::from("pairwise"),
            Variant(Box::new(vec![String::from("ccmp")]) as Box<dyn RefArg>),
        ),
        (
            String::from("group"),
            Variant(Box::new(vec![String::from("ccmp")]) as Box<dyn RefArg>),
        ),
    ])
}

fn at_the_owner_address(group: &WifiGroup) -> Section {
    let mut address: HashMap<String, Variant<Box<dyn RefArg>>> = HashMap::new();
    address.insert(
        String::from("address"),
        Variant(Box::new(group.owner_address.to_string()) as Box<dyn RefArg>),
    );
    address.insert(
        String::from("prefix"),
        Variant(Box::new(24u32) as Box<dyn RefArg>),
    );
    Section::from([
        (String::from("method"), text("manual")),
        (
            String::from("address-data"),
            Variant(Box::new(vec![address]) as Box<dyn RefArg>),
        ),
        (
            String::from("never-default"),
            Variant(Box::new(true) as Box<dyn RefArg>),
        ),
    ])
}
