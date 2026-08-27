//! A node configured for covert must come up whether or not this host can
//! open the carrier's listening socket — an ICMP server needs raw sockets, and
//! most hosts have none. Losing the listener must cost the node nothing else.

use resilum_core::{Config, CovertDiscoveryService, Node};

fn covert_node(dir: &std::path::Path) -> Node {
    Node::new(Config {
        storage_path: Some(dir.to_path_buf()),
        discover_interfaces: false,
        listen: Some("127.0.0.1:0".into()),
        covert_discovery: vec![CovertDiscoveryService::icmp()],
        ..Config::minimal(format!("covert-{}", std::process::id()))
    })
    .expect("new")
}

#[test]
fn a_node_that_cannot_listen_covertly_still_starts_and_stops() {
    let dir = std::env::temp_dir().join(format!("resilum-covert-{}", std::process::id()));

    let mut node = covert_node(&dir);
    let started = node.start();
    let stopped = node.stop();
    let _ = std::fs::remove_dir_all(&dir);

    started.expect("a covert node starts even where it cannot listen");
    stopped.expect("nothing outlives stop still holding the engine");
}
