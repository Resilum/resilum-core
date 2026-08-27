use proptest::prelude::*;

use super::*;

fn ip(text: &str) -> IpAddr {
    text.parse().expect("test address")
}

#[test]
fn rejects_private_v4() {
    for text in [
        "10.0.0.1",
        "172.16.0.1",
        "192.168.1.1",
        "127.0.0.1",
        "169.254.1.1",
        "100.64.0.1",
        "0.1.2.3",
        "255.255.255.255",
        "240.0.0.1",
    ] {
        assert!(!is_globally_routable(&ip(text)), "{text} must be filtered");
    }
}

#[test]
fn accepts_public_v4() {
    for text in ["198.18.0.1", "198.19.255.254", "203.0.113.1"] {
        // 203.0.113.0/24 is TEST-NET-3 (documentation): expected to be filtered.
        let accepted = is_globally_routable(&ip(text));
        if text == "203.0.113.1" {
            assert!(!accepted, "TEST-NET-3 must be filtered as documentation");
        } else {
            assert!(accepted, "{text} must be global");
        }
    }
}

#[test]
fn rejects_private_v6() {
    for text in [
        "::1",
        "fe80::1",
        "fc00::1",
        "fd00::1",
        "ff02::1",
        "::",
        "::ffff:198.18.0.1",
    ] {
        assert!(!is_globally_routable(&ip(text)), "{text} must be filtered");
    }
}

#[test]
fn accepts_public_v6() {
    for text in ["2606:4700:4700::1111", "2001:db8::1"] {
        // 2001:db8::/32 is documentation but not filtered here — it is not
        // link-local/ULA/multicast, and we intentionally don't reject it
        // (users may run into it in test networks).
        assert!(is_globally_routable(&ip(text)), "{text} should pass");
    }
}

#[test]
fn a_local_network_address_names_a_host_even_though_it_is_not_global() {
    for text in [
        "192.168.1.1",
        "10.0.0.1",
        "169.254.1.1",
        "fd00::1",
        "fe80::1",
    ] {
        assert!(names_one_host(&ip(text)), "{text} names a reachable host");
        assert!(!is_globally_routable(&ip(text)), "{text} is not global");
    }
}

#[test]
fn this_host_no_host_and_every_host_never_name_one_host() {
    for text in [
        "127.0.0.1",
        "0.0.0.0",
        "255.255.255.255",
        "224.0.0.1",
        "::1",
        "::",
        "ff02::1",
        "::ffff:198.18.0.1",
    ] {
        assert!(!names_one_host(&ip(text)), "{text} must never be dialed");
    }
}

proptest! {
    #[test]
    fn never_panics_on_arbitrary_v4(bytes in any::<[u8; 4]>()) {
        let addr = IpAddr::V4(Ipv4Addr::from(bytes));
        let _ = is_globally_routable(&addr);
        let _ = names_one_host(&addr);
    }
    #[test]
    fn never_panics_on_arbitrary_v6(bytes in any::<[u8; 16]>()) {
        let addr = IpAddr::V6(Ipv6Addr::from(bytes));
        let _ = is_globally_routable(&addr);
        let _ = names_one_host(&addr);
    }

    #[test]
    fn a_globally_routable_address_always_names_one_host_v4(bytes in any::<[u8; 4]>()) {
        let addr = IpAddr::V4(Ipv4Addr::from(bytes));
        prop_assert!(!is_globally_routable(&addr) || names_one_host(&addr));
    }
    #[test]
    fn a_globally_routable_address_always_names_one_host_v6(bytes in any::<[u8; 16]>()) {
        let addr = IpAddr::V6(Ipv6Addr::from(bytes));
        prop_assert!(!is_globally_routable(&addr) || names_one_host(&addr));
    }
}
