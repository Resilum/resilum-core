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
    let dir = tempfile::tempdir().expect("a temporary directory");

    let mut node = node_listening_on(port, dir.path());
    node.start().expect("first start");
    node.stop().expect("stop");

    let mut again = node_listening_on(port, dir.path());
    let second = again.start();
    again.stop().ok();

    second.expect("the listener port to be free again");
}
