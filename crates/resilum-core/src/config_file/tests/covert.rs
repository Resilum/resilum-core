use super::from_yaml;
use crate::discovery::covert::Reach;

#[test]
fn covert_section_carries_the_carrier_addresses_and_reach() {
    let cfg = from_yaml(
        "
instance_name: n
covert:
  - carrier: icmp
    mtu: 1200
    addresses: ['198.18.0.1']
    dial_local_networks: true
",
    )
    .unwrap();
    let covert = &cfg.covert_discovery[0];
    assert_eq!(covert.carrier, "icmp");
    assert_eq!(covert.mtu, 1200);
    assert_eq!(covert.addresses, vec!["198.18.0.1"]);
    assert_eq!(covert.reach(), Reach::LocalNetworksToo);
}

#[test]
fn a_node_on_the_default_network_carries_icmp_without_being_asked() {
    let cfg = from_yaml("instance_name: n").unwrap();
    let carriers: Vec<&str> = cfg
        .covert_discovery
        .iter()
        .map(|c| c.carrier.as_str())
        .collect();

    assert_eq!(carriers, vec!["icmp"]);
}

#[test]
fn covert_keeps_to_globally_routable_peers_unless_asked_otherwise() {
    let cfg = from_yaml(
        "
instance_name: n
covert:
  - carrier: icmp
",
    )
    .unwrap();
    assert_eq!(cfg.covert_discovery[0].reach(), Reach::GlobalOnly);
}

#[test]
fn an_isolated_node_carries_nothing_it_was_not_given() {
    let cfg = from_yaml(
        "
instance_name: n
default_anchors: false
",
    )
    .unwrap();

    assert!(cfg.covert_discovery.is_empty());
}

#[test]
fn an_empty_covert_list_turns_the_default_icmp_off() {
    let cfg = from_yaml(
        "
instance_name: n
covert: []
",
    )
    .unwrap();

    assert!(cfg.covert_discovery.is_empty());
}
