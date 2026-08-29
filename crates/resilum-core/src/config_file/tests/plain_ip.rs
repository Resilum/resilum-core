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
",
    )
    .unwrap();

    assert!(!cfg.bootstrap.is_empty());
}
