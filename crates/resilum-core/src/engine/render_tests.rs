use super::render_config;
use crate::Config;

const A_PORT_WE_CHOSE: u16 = 51234;

#[test]
fn renders_listener_bootstrap_and_autoconnect() {
    let cfg = Config {
        listen: Some("[::]:4242".into()),
        bootstrap: vec!["anchor.example:4343".into()],
        ..Config::minimal("test")
    };
    let ini = render_config(&cfg, A_PORT_WE_CHOSE);
    assert!(ini.contains("autoconnect_discovered_interfaces = 5"));
    assert!(ini.contains("control_channel_capacity = 16384"));
    assert!(ini.contains("type = AutoInterface"));
    assert!(ini.contains(&format!("data_port = {A_PORT_WE_CHOSE}")));
    assert!(ini.contains("listen_ip = [::]"));
    assert!(ini.contains("listen_port = 4242"));
    assert!(ini.contains("target_host = anchor.example"));
    assert!(ini.contains("target_port = 4343"));
}

#[test]
fn no_discovery_disables_autoconnect_and_auto_interface() {
    let cfg = Config {
        discover_interfaces: false,
        ..Config::minimal("x")
    };
    let ini = render_config(&cfg, A_PORT_WE_CHOSE);
    assert!(ini.contains("discover_interfaces = no"));
    assert!(ini.contains("autoconnect_discovered_interfaces = 0"));
    assert!(!ini.contains("AutoInterface"));
}

#[test]
fn default_network_renders_anchors_and_discovery() {
    let ini = render_config(&Config::default_network("node"), A_PORT_WE_CHOSE);
    assert!(ini.contains("listen_port = 4242"));
    assert!(ini.contains("discovery_name = resilum"));
    assert!(ini.contains("network_identity = network_identity"));
    assert!(ini.contains("target_host = istanbul.reserve.network"));
    assert!(ini.contains("bootstrap_only = yes"));
    assert!(ini.contains("target_host = [200:3953:999b:282e:e526:bcd2:c329:31a]"));
}

#[test]
fn renders_udp_beside_the_tcp_interfaces() {
    let cfg = Config {
        udp: Some(crate::config::UdpInterface {
            listen: Some("0.0.0.0:4343".into()),
            peers_every_datagram_goes_to: vec!["a.example:4242".into(), "b.example:4242".into()],
            reachable_on: Vec::new(),
        }),
        ..Config::minimal("test")
    };

    let ini = render_config(&cfg, A_PORT_WE_CHOSE);

    assert!(ini.contains("type = UDPInterface"));
    assert!(ini.contains("listen_ip = 0.0.0.0"));
    assert!(ini.contains("listen_port = 4343"));
    assert!(ini.contains("forward_ip = a.example:4242, b.example:4242"));
}

#[test]
fn a_udp_interface_with_nobody_to_forward_to_is_not_rendered() {
    let cfg = Config {
        udp: Some(crate::config::UdpInterface::default()),
        ..Config::minimal("test")
    };

    assert!(!render_config(&cfg, A_PORT_WE_CHOSE).contains("UDPInterface"));
}

#[test]
fn renders_i2p_interface() {
    let cfg = Config {
        i2p: Some(crate::config::I2pInterface {
            connectable: true,
            peers: vec!["a.b32.i2p".into()],
        }),
        ..Config::minimal("test")
    };
    let ini = render_config(&cfg, A_PORT_WE_CHOSE);
    assert!(ini.contains("type = I2PInterface"));
    assert!(ini.contains("connectable = yes"));
    assert!(ini.contains("peers = a.b32.i2p"));
}

#[test]
fn covert_spec_renders_a_pipe_interface() {
    let mut cfg = Config::minimal("test");
    cfg.specs =
        crate::spec::load("covert:\n  - carrier: icmp\n    command: rns-over-icmp --peer x\n")
            .unwrap();
    let ini = render_config(&cfg, A_PORT_WE_CHOSE);
    assert!(ini.contains("[[covert/icmp]]"));
    assert!(ini.contains("type = PipeInterface"));
    assert!(ini.contains("command = rns-over-icmp --peer x"));
}

fn under(ini: &str, heading: &str) -> String {
    let from = ini.find(heading).expect("the section was rendered");
    let section = &ini[from + heading.len()..];
    let to = section.find("\n  [[").unwrap_or(section.len());
    section[..to].to_owned()
}

#[test]
fn every_way_out_to_the_wider_network_is_a_boundary() {
    let mut cfg = Config {
        listen: Some("[::]:4242".into()),
        bootstrap: vec!["anchor.example:4343".into()],
        bootstrap_only: vec!["only.example:4242".into()],
        i2p: Some(crate::config::I2pInterface {
            connectable: false,
            peers: Vec::new(),
        }),
        ..Config::minimal("modes")
    };
    cfg.specs = crate::spec::load("covert:\n  - carrier: icmp\n    command: c\n").unwrap();

    let ini = render_config(&cfg, A_PORT_WE_CHOSE);

    assert!(under(&ini, "[[Bootstrap 0]]").contains("mode = boundary"));
    assert!(under(&ini, "[[Bootstrap-only 0]]").contains("mode = boundary"));
    assert!(under(&ini, "[[I2P]]").contains("mode = boundary"));
    assert!(under(&ini, "[[Public TCP listener]]").contains("mode = gateway"));
    assert!(
        !under(&ini, "[[LAN AutoDiscovery]]").contains("mode ="),
        "the LAN is well connected and stays full"
    );
    assert!(
        !under(&ini, "[[covert/icmp]]").contains("mode ="),
        "a slow carrier is capped, not cut off from transit announces"
    );
}
