use std::net::Ipv4Addr;

#[derive(Clone, Debug)]
pub struct WifiGroup {
    pub ssid: String,
    pub passphrase: String,
    pub interface: Option<String>,
    pub owner_address: Ipv4Addr,
    pub port: u16,
}

impl Default for WifiGroup {
    fn default() -> Self {
        Self {
            ssid: String::from("DIRECT-RS-resilum"),
            passphrase: String::from("resilum-open-mesh"),
            interface: None,
            owner_address: Ipv4Addr::new(192, 168, 49, 1),
            port: 4242,
        }
    }
}
