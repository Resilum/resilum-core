use std::time::Duration;

use resilum_core::{Config, Node};

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn node_starts_on_real_engine() {
    let dir = temp_dir();
    let cfg = Config {
        storage_path: Some(dir.path().to_path_buf()),
        discover_interfaces: false,
        ..Config::minimal(format!("smoke-bare-{}", std::process::id()))
    };

    let mut node = Node::new(cfg).expect("new");
    let started = node.start();
    eprintln!("start() -> {started:?}");
    assert!(
        started.is_ok(),
        "engine rejected rendered config: {started:?}"
    );
    assert!(node.is_running());

    let engine = node.engine().expect("engine handle after start");
    assert!(engine.is_running(), "shared engine handle drives &self ops");
    drop(engine);

    node.stop().expect("stop");
    assert!(node.engine().is_none(), "engine handle cleared after stop");
}

#[test]
fn tcp_listener_from_config_binds() {
    let dir = temp_dir();

    let (mut node, port) = (0..10)
        .find_map(|_| {
            let port = free_port();
            let cfg = Config {
                storage_path: Some(dir.path().to_path_buf()),
                discover_interfaces: false,
                listen: Some(format!("127.0.0.1:{port}")),
                ..Config::minimal(format!("smoke-listen-{}", std::process::id()))
            };
            let mut node = Node::new(cfg).expect("new");
            node.start().is_ok().then_some((node, port))
        })
        .expect("engine started with a listener");

    let mut connected = false;
    for _ in 0..40 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            connected = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(connected, "TCP listener from config did not come up");

    node.stop().expect("stop");
}

#[test]
fn a_node_whose_anchor_name_will_not_resolve_still_starts() {
    let dir = temp_dir();
    let cfg = Config {
        storage_path: Some(dir.path().to_path_buf()),
        discover_interfaces: false,
        bootstrap: vec!["there-is-no-such-host.invalid:9034".into()],
        ..Config::minimal(format!("smoke-nameless-{}", std::process::id()))
    };

    let mut node = Node::new(cfg).expect("new");

    node.start()
        .expect("a name that will not resolve costs its own anchor, not the node");

    node.stop().expect("stop");
}
