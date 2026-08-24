//! A node stopped and started again must get its own ports back. The mobile
//! app restarts within a couple of hundred milliseconds when a start with Tor
//! fails and it falls back to one without.

use resilum_core::{Config, Node};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a local port")
        .local_addr()
        .expect("the port just bound")
        .port()
}

fn node_listening_on(port: u16, dir: &std::path::Path) -> Node {
    Node::new(Config {
        storage_path: Some(dir.to_path_buf()),
        discover_interfaces: false,
        listen: Some(format!("127.0.0.1:{port}")),
        ..Config::minimal(format!("restart-{}", std::process::id()))
    })
    .expect("new")
}

#[test]
fn the_port_is_free_the_moment_stop_returns() {
    let port = free_port();
    let dir = std::env::temp_dir().join(format!("resilum-restart-{}", std::process::id()));

    let mut node = node_listening_on(port, &dir);
    node.start().expect("first start");
    node.stop().expect("stop");

    let mut again = node_listening_on(port, &dir);
    let second = again.start();
    again.stop().ok();
    let _ = std::fs::remove_dir_all(&dir);

    second.expect("the listener port to be free again");
}
