use super::from_yaml;

#[test]
fn maps_egress_and_connect() {
    let cfg = from_yaml(
        "
instance_name: node-a
default_anchors: false
listen: '[::]:4242'
bootstrap: [anchor.example:4343]
egress:
  - service: socks-egress
    target: 127.0.0.1:1080
    exit_country: DE
  - service: tor
    target: 127.0.0.1:9050
ingress:
  services: [socks-egress, tor]
  listen_tcp: 127.0.0.1:10808
",
    )
    .unwrap();
    assert_eq!(cfg.instance_name, "node-a");
    assert_eq!(cfg.bootstrap, vec!["anchor.example:4343"]);
    assert_eq!(cfg.egress.len(), 2);
    assert_eq!(cfg.egress[0].exit_country, "DE");
    assert_eq!(cfg.egress[1].service, "tor");
    assert_eq!(cfg.ingress.unwrap().use_own, "smart");
}

#[test]
fn parses_discovery_with_socks_override() {
    use crate::config::{EndpointFormat, SocksProxy};
    let cfg = from_yaml(
        "
instance_name: n
discovery:
  - service: tor
    socks: '127.0.0.1:9051'
  - service: i2p
  - service: yggdrasil
",
    )
    .unwrap();
    assert_eq!(cfg.discovery.len(), 3);
    assert!(
        matches!(cfg.discovery[0].endpoint_format, EndpointFormat::Suffix(ref s) if s == ".onion")
    );
    assert!(
        matches!(&cfg.discovery[0].socks_proxy, Some(SocksProxy::External(h, 9051)) if h == "127.0.0.1")
    );
    assert_eq!(cfg.discovery[1].service, "i2p");
    assert!(matches!(
        cfg.discovery[2].endpoint_format,
        EndpointFormat::BracketedIpv6
    ));
}

#[test]
fn bare_config_joins_the_default_network() {
    let cfg = from_yaml("instance_name: bare").unwrap();
    assert!(cfg.egress.is_empty());
    assert!(!cfg.bootstrap.is_empty());
    assert!(cfg.listen.is_some());
}
