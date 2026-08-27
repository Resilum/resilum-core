use super::from_yaml;
use crate::discovery::covert::Reach;

#[test]
fn covert_section_carries_the_carrier_addresses_and_reach() {
    let cfg = from_yaml(
        "
instance_name: n
covert:
  - identity_path: /var/lib/resilum/covert.id
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
fn covert_keeps_to_globally_routable_peers_unless_asked_otherwise() {
    let cfg = from_yaml(
        "
instance_name: n
covert:
  - identity_path: /var/lib/resilum/covert.id
",
    )
    .unwrap();
    assert_eq!(cfg.covert_discovery[0].reach(), Reach::GlobalOnly);

    assert!(
        from_yaml("instance_name: n")
            .unwrap()
            .covert_discovery
            .is_empty()
    );
}
