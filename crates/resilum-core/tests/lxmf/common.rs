//! Shared setup for the LXMF tests.

use std::path::Path;
use std::time::{Duration, Instant};

use data_encoding::HEXLOWER;
use resilum_core::{Config, LxmfConfig, Node};
use serde_json::Value;

const PATIENCE: Duration = Duration::from_secs(60);

pub fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("resilum-lxmf-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A started node with messaging on, waited until its router has registered.
pub fn start(tag: &str, dir: &Path, listen: Option<u16>, bootstrap: Vec<String>) -> Node {
    let config = Config {
        storage_path: Some(dir.to_path_buf()),
        discover_interfaces: false,
        listen: listen.map(|p| format!("127.0.0.1:{p}")),
        bootstrap,
        lxmf: Some(LxmfConfig::default()),
        ..Config::minimal(format!("lxmf-{tag}-{}", std::process::id()))
    };
    let mut node = Node::new(config).expect("new");
    node.start().expect("start");
    wait_for("the router to register", || {
        node.lxmf()?.is_ready().then_some(())
    });
    node
}

/// Build `request` as this node and hand it to the router, returning the
/// message id the app would have got back.
pub fn submit(node: &Node, request: &str) -> String {
    let identity = node.identity().expect("identity").clone();
    let source_hash = resilum_core::identity::lxmf_address(&identity);
    let message = resilum_core::lxmf::send::build_message(request, &identity, source_hash, 1.0)
        .expect("build");
    let message_id = HEXLOWER.encode(&message.message_id);
    node.lxmf()
        .expect("messaging")
        .submit(message)
        .expect("submit");
    message_id
}

/// Wait for the next event of `kind` out of the node's poll queue.
pub fn next_event(node: &Node, kind: &str, what: &str) -> Value {
    wait_for(what, || {
        let json = node.lxmf()?.next_event()?;
        let value: Value = serde_json::from_str(&json).ok()?;
        (value["type"] == kind).then_some(value)
    })
}

/// Poll `probe` until it yields, or fail naming what never happened.
///
/// Announces, paths and deliveries arrive on the mesh's own timing, so a test
/// that reads once reads too early. `what` is what the failure message says —
/// the difference between a diagnosable timeout and "assertion failed".
pub fn wait_for<T>(what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        if let Some(value) = probe() {
            return value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("timed out waiting for {what}");
}
