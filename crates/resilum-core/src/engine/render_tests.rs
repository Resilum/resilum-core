use super::render_config;
use crate::Config;

#[test]
fn renders_listener_bootstrap_and_autoconnect() {
    let cfg = Config {
        listen: Some("[::]:4242".into()),
        bootstrap: vec!["anchor.example:4343".into()],
        ..Config::minimal("test")
    };
    let ini = render_config(&cfg);
    assert!(ini.contains("autoconnect_discovered_interfaces = 5"));
    assert!(ini.contains("control_channel_capacity = 16384"));
    assert!(ini.contains("type = AutoInterface"));
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
    let ini = render_config(&cfg);
    assert!(ini.contains("discover_interfaces = no"));
    assert!(ini.contains("autoconnect_discovered_interfaces = 0"));
    assert!(!ini.contains("AutoInterface"));
}

#[test]
fn default_network_renders_anchors_and_discovery() {
    let ini = render_config(&Config::default_network("node"));
    assert!(ini.contains("listen_port = 4242"));
    assert!(ini.contains("discovery_name = resilum"));
    assert!(ini.contains("network_identity = network_identity"));
    assert!(ini.contains("target_host = istanbul.reserve.network"));
    assert!(ini.contains("bootstrap_only = yes"));
    assert!(ini.contains("target_host = [200:3953:999b:282e:e526:bcd2:c329:31a]"));
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
    let ini = render_config(&cfg);
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
    let ini = render_config(&cfg);
    assert!(ini.contains("[[covert/icmp]]"));
    assert!(ini.contains("type = PipeInterface"));
    assert!(ini.contains("command = rns-over-icmp --peer x"));
}
