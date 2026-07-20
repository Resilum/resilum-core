//! Black-box integration test: exercises only the public `Node` API.

use resilum_core::{Config, Node};

#[test]
fn a_fresh_node_is_idle() {
    let mut node = Node::new(Config::minimal("test")).expect("build node");
    assert!(!node.is_running());
    assert!(node.poll_event().is_none());
    // send is rejected until the node is started
    assert!(node.send(b"dest", b"data").is_err());
}
