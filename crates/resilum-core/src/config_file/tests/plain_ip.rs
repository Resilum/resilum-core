use super::from_yaml;

#[test]
fn turning_off_plain_ip_drops_the_listener_public_anchors_and_lan() {
    let cfg = from_yaml(
        "
instance_name: n
plain_ip: false
",
    )
    .unwrap();

    assert!(cfg.listen.is_none());
    assert!(cfg.bootstrap_only.is_empty());
    assert!(!cfg.discover_interfaces);
}

#[test]
fn a_udp_section_carries_its_listener_and_peers() {
    let cfg = from_yaml(
        "
instance_name: n
udp:
  listen: '0.0.0.0:4343'
  peers: [a.example:4242]
",
    )
    .unwrap();

    let udp = cfg.udp.expect("udp interface");
    assert_eq!(udp.bound_to(), "0.0.0.0:4343");
    assert_eq!(udp.peers_every_datagram_goes_to, vec!["a.example:4242"]);
}

#[test]
fn udp_is_plain_ip_so_the_switch_takes_it_too() {
    let cfg = from_yaml(
        "
instance_name: n
plain_ip: false
udp:
  peers: [a.example:4242]
",
    )
    .unwrap();

    assert!(cfg.udp.is_none());
}

#[test]
fn a_ble_section_says_whether_this_host_can_host_a_group() {
    let cfg = from_yaml(
        "
instance_name: n
ble:
  can_host_a_group: true
",
    )
    .unwrap();

    assert!(cfg.ble.expect("ble interface").can_host_a_group);
}

#[test]
fn ble_is_its_own_medium_so_the_plain_ip_switch_leaves_it_alone() {
    let cfg = from_yaml(
        "
instance_name: n
plain_ip: false
ble: {}
",
    )
    .unwrap();

    assert!(cfg.ble.is_some());
}

#[test]
fn a_config_that_never_mentions_ble_does_not_open_a_radio() {
    let cfg = from_yaml("instance_name: n").unwrap();

    assert!(cfg.ble.is_none());
}

#[test]
fn plain_ip_leaves_the_ygg_anchors_to_the_ygg_transport() {
    let cfg = from_yaml(
        "
instance_name: n
plain_ip: false
discovery:
  - service: yggdrasil
",
    )
    .unwrap();

    assert!(!cfg.bootstrap.is_empty());
}

#[test]
fn without_the_ygg_transport_its_anchors_are_not_dialled() {
    let cfg = from_yaml(
        "
instance_name: n
discovery:
  - service: tor
",
    )
    .unwrap();

    assert!(cfg.bootstrap.is_empty());
}

#[test]
fn own_anchors_outlive_the_ygg_transport_being_off() {
    let cfg = from_yaml(
        "
instance_name: n
bootstrap: [anchor.example:4343]
discovery:
  - service: tor
",
    )
    .unwrap();

    assert_eq!(cfg.bootstrap, vec!["anchor.example:4343"]);
}
