//! Which addresses the embedded SOCKS backend is allowed to dial.

use std::net::SocketAddr;

/// Every resolved address must pass: a name that resolves to both a public and
/// a private address is a rebinding attempt, not a partially valid target.
pub(super) fn dialable(addrs: &[SocketAddr], allow_private: bool) -> bool {
    if addrs.is_empty() {
        return false;
    }
    allow_private
        || addrs
            .iter()
            .all(|a| crate::net::is_globally_routable(&a.ip()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addrs(list: &[&str]) -> Vec<SocketAddr> {
        list.iter().map(|a| a.parse().unwrap()).collect()
    }

    #[test]
    fn the_node_own_services_are_not_dialable() {
        for a in [
            "127.0.0.1:9050",
            "127.0.0.1:7656",
            "[::1]:4242",
            "10.0.0.5:22",
            "192.168.1.1:80",
            "172.16.0.1:80",
            "[fd00::1]:80",
        ] {
            assert!(!dialable(&addrs(&[a]), false), "{a} must be refused");
        }
    }

    #[test]
    fn cloud_metadata_is_not_dialable() {
        assert!(!dialable(&addrs(&["169.254.169.254:80"]), false));
    }

    #[test]
    fn public_addresses_are_dialable() {
        assert!(dialable(&addrs(&["198.18.0.1:443"]), false));
        assert!(dialable(&addrs(&["[2001:2::1]:443"]), false));
    }

    #[test]
    fn one_private_answer_refuses_the_whole_name() {
        assert!(!dialable(
            &addrs(&["198.18.0.1:443", "127.0.0.1:443"]),
            false
        ));
    }

    #[test]
    fn a_name_that_resolves_to_nothing_is_not_dialable() {
        assert!(!dialable(&[], false));
    }

    #[test]
    fn the_operator_can_allow_private_targets() {
        assert!(dialable(&addrs(&["10.0.0.5:22"]), true));
        assert!(dialable(&addrs(&["198.18.0.1:443", "127.0.0.1:443"]), true));
        assert!(!dialable(&[], true));
    }
}
