//! Black-box test of the config-spec loader.

use resilum_core::spec::{self, BridgeMode};

#[test]
fn loads_bridges_vpn_and_covert() {
    let yaml = r#"
bridges:
  - mode: listen
    service: yggdrasil
    identity: /config/bridges/yggdrasil.id
    tcp: 127.0.0.1:9000
  - mode: connect
    services: [socks-egress, tor, i2p]
    identity: /config/bridges/socks-egress-out.id
    tcp: 127.0.0.1:10808
vpn:
  - mode: server
    identity: /config/vpn/server.id
    subnet: 10.20.0.0/24
    mtu: 1280
covert:
  - carrier: icmp
    addresses: "192.0.2.1, 192.0.2.2"
"#;
    let specs = spec::load(yaml).expect("load");
    assert_eq!(specs.bridges.len(), 2);
    assert_eq!(specs.bridges[0].mode, BridgeMode::Listen);
    assert_eq!(specs.bridges[0].services, ["yggdrasil"]);
    assert_eq!(specs.bridges[1].services.len(), 3);
    assert_eq!(specs.vpn.len(), 1);
    assert_eq!(specs.covert[0].addresses.len(), 2);
}

#[test]
fn listen_with_multiple_services_is_rejected() {
    let yaml = "\
bridges:
  - mode: listen
    services: [a, b]
    identity: x
    tcp: 127.0.0.1:9000
";
    assert!(spec::load(yaml).is_err());
}
