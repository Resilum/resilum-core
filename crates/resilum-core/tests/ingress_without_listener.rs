use std::time::Duration;

use resilum_core::{Config, IngressConfig, Node};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a local port")
        .local_addr()
        .expect("the port just bound")
        .port()
}

fn node_with(ingress: IngressConfig, tag: &str) -> (Node, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("resilum-{tag}-{}", std::process::id()));
    let node = Node::new(Config {
        storage_path: Some(dir.clone()),
        discover_interfaces: false,
        ingress: Some(ingress),
        ..Config::minimal(format!("{tag}-{}", std::process::id()))
    })
    .expect("new");
    (node, dir)
}

#[test]
fn an_ingress_without_a_listen_address_binds_nothing() {
    let (mut node, dir) = node_with(IngressConfig::without_a_listener("e2e"), "no-listener");
    node.start().expect("start");
    std::thread::sleep(Duration::from_millis(200));

    let port = node.socks_port();

    node.stop().ok();
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(port, None, "a node with no listener reported a port");
}

#[test]
fn an_ingress_with_one_still_binds_it() {
    let asked_for = free_port();
    let (mut node, dir) = node_with(
        IngressConfig::without_a_listener("e2e").listening_on(format!("127.0.0.1:{asked_for}")),
        "listener",
    );
    node.start().expect("start");

    let bound = (0..40).find_map(|_| match node.socks_port() {
        Some(port) if port != 0 => Some(port),
        _ => {
            std::thread::sleep(Duration::from_millis(50));
            None
        }
    });

    node.stop().ok();
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(bound, Some(asked_for));
}
