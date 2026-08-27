//! Effective address list for a covert carrier: explicit config wins;
//! otherwise, auto-detected globally-routable local egress addresses.

use std::fmt;
use std::net::IpAddr;

use crate::net;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    GlobalOnly,
    LocalNetworksToo,
}

pub struct DialableAddress(IpAddr);

impl DialableAddress {
    pub fn first_of(offered: &[String], reach: Reach) -> Option<Self> {
        offered
            .iter()
            .filter_map(|addr| addr.parse::<IpAddr>().ok())
            .find(|ip| match reach {
                Reach::GlobalOnly => net::is_globally_routable(ip),
                Reach::LocalNetworksToo => net::names_one_host(ip),
            })
            .map(Self)
    }

    pub fn ip(&self) -> IpAddr {
        self.0
    }
}

impl fmt::Display for DialableAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub struct AddressSource {
    configured: Vec<String>,
    detected: Vec<String>,
}

impl AddressSource {
    pub fn new(configured: Vec<String>) -> Self {
        let detected = if configured.is_empty() {
            net::local_egress_addresses()
                .into_iter()
                .map(|ip| ip.to_string())
                .collect()
        } else {
            Vec::new()
        };
        Self {
            configured,
            detected,
        }
    }

    pub fn effective(&self) -> &[String] {
        if !self.configured.is_empty() {
            &self.configured
        } else {
            &self.detected
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered(addrs: &[&str]) -> Vec<String> {
        addrs.iter().map(|a| (*a).to_owned()).collect()
    }

    #[test]
    fn an_address_pointing_back_at_this_host_or_its_network_is_passed_over() {
        for addr in [
            "127.0.0.1",
            "10.0.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "255.255.255.255",
            "224.0.0.1",
            "::1",
            "fe80::1",
            "fd00::1",
        ] {
            assert!(
                DialableAddress::first_of(&offered(&[addr]), Reach::GlobalOnly).is_none(),
                "{addr} was accepted as dialable"
            );
        }
    }

    #[test]
    fn a_routable_address_further_down_the_list_is_still_reached() {
        let chosen =
            DialableAddress::first_of(&offered(&["127.0.0.1", "198.18.0.1"]), Reach::GlobalOnly)
                .expect("the routable address is picked");

        assert_eq!(chosen.ip().to_string(), "198.18.0.1");
    }

    #[test]
    fn opening_up_to_local_networks_reaches_a_lan_peer() {
        let chosen =
            DialableAddress::first_of(&offered(&["192.168.0.10"]), Reach::LocalNetworksToo)
                .expect("a LAN peer is reachable once local networks are allowed");

        assert_eq!(chosen.ip().to_string(), "192.168.0.10");
    }

    #[test]
    fn opening_up_to_local_networks_still_refuses_this_host_and_broadcasts() {
        for addr in [
            "127.0.0.1",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "::1",
        ] {
            assert!(
                DialableAddress::first_of(&offered(&[addr]), Reach::LocalNetworksToo).is_none(),
                "{addr} was accepted once local networks were allowed"
            );
        }
    }
}
