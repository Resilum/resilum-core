//! Runtime interface attach/detach through the resilum-core `Node` facade: a
//! UDP interface added from a leviculum interface-config JSON appears in engine
//! status, then removes cleanly. Pins the JSON contract the FFI exposes.

use std::time::Duration;

use resilum_core::{Config, Node};

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

fn status_has(node: &Node, name: &str) -> bool {
    let engine = node.engine().expect("engine");
    (0..50).any(|_| {
        if engine.interface_stats().iter().any(|i| i.name == name) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
        false
    })
}

fn status_lacks(node: &Node, name: &str) -> bool {
    let engine = node.engine().expect("engine");
    (0..50).any(|_| {
        if engine.interface_stats().iter().all(|i| i.name != name) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
        false
    })
}

#[test]
fn add_and_remove_interface_from_json() {
    let dir = temp_dir();
    let mut node = Node::new(Config {
        storage_path: Some(dir.path().to_path_buf()),
        discover_interfaces: false,
        ..Config::minimal(format!("iar-{}", std::process::id()))
    })
    .expect("new");
    node.start().expect("start");

    let cfg = r#"{"type":"UDPInterface","listen_ip":"127.0.0.1","listen_port":0,
                  "forward_ip":"127.0.0.1","forward_port":37000}"#;
    let ids = node.add_interface(cfg).expect("add_interface");
    assert_eq!(ids.len(), 1, "UDP is a single-handle interface");
    let name = format!("udp_{}", ids[0]);

    assert!(
        status_has(&node, &name),
        "added interface appears in status"
    );

    node.remove_interface(ids[0]).expect("remove_interface");
    assert!(
        status_lacks(&node, &name),
        "removed interface is gone from status"
    );

    node.stop().expect("stop");
}
