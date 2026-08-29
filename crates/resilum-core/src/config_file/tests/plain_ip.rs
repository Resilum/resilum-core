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
