use std::ffi::{CStr, CString};
use std::time::{Duration, Instant};

use resilum_ffi::*;
use serde_json::Value;

const PATIENCE: Duration = Duration::from_secs(30);

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

fn config(tag: &str, dir: &std::path::Path, lxmf: &str) -> CString {
    CString::new(format!(
        "instance_name: ffi-status-lxmf-{tag}\ndefault_anchors: false\ndiscover_interfaces: false\nstorage_path: \"{}\"\n{lxmf}",
        dir.display()
    ))
    .expect("config has no interior NUL")
}

fn status_json(node: *const ResilumNode) -> Value {
    let raw = unsafe { resilum_node_status(node) };
    assert!(!raw.is_null());
    let text = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_owned();
    unsafe { resilum_string_free(raw) };
    serde_json::from_str(&text).expect("valid json")
}

fn wait_until(deadline: Instant, what: &str, mut ready: impl FnMut() -> bool) {
    while !ready() {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Queues one message the isolated node can never deliver, so `queued_count`
/// and `queued_ids` below compare something other than zero to zero.
fn submit_undeliverable(node: *mut ResilumNode) {
    let request = CString::new(
        r#"{"destination":"00112233445566778899aabbccddeeff","method":"direct","content_b64":"aGk="}"#,
    )
    .expect("request has no interior NUL");
    let id = unsafe { resilum_lxmf_send(node, request.as_ptr()) };
    assert!(!id.is_null(), "send: {}", last_error());
    unsafe { resilum_string_free(id) };
}

fn last_error() -> String {
    let raw = resilum_last_error();
    if raw.is_null() {
        return "(no error)".into();
    }
    unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn lxmf_section_matches_the_dedicated_symbols_once_ready() {
    let dir = temp_dir();
    let node =
        unsafe { resilum_node_new_from_yaml(config("on", dir.path(), "lxmf: {}\n").as_ptr()) };
    assert!(!node.is_null());
    assert_eq!(unsafe { resilum_node_start(node) }, RESILUM_OK);

    let deadline = Instant::now() + PATIENCE;
    wait_until(deadline, "router never registered", || {
        (unsafe { resilum_lxmf_is_ready(node) }) != 0
    });
    submit_undeliverable(node);
    wait_until(
        deadline,
        "submitted message never reached the queue",
        || (unsafe { resilum_lxmf_queued_count(node) }) > 0,
    );

    let addr_ptr = unsafe { resilum_lxmf_address(node) };
    assert!(!addr_ptr.is_null());
    let address = unsafe { CStr::from_ptr(addr_ptr) }
        .to_str()
        .unwrap()
        .to_owned();
    unsafe { resilum_string_free(addr_ptr) };
    let queued_symbol = unsafe { resilum_lxmf_queued_count(node) };
    assert!(queued_symbol > 0, "the queue must be non-empty to compare");

    let status = status_json(node);
    let lxmf = &status["lxmf"];
    assert_eq!(lxmf["ready"], true);
    assert_eq!(lxmf["address"], address);
    assert_eq!(address.len(), 32, "address: {address}");
    assert_eq!(lxmf["queued_count"], queued_symbol);
    // Its length is the same sample `queued_count` was taken from.
    let queued_ids = lxmf["queued_ids"].as_array().expect("queued_ids array");
    assert_eq!(queued_ids.len() as u64, queued_symbol);
    // No interfaces and no bootstrap: this node can never hear a propagation
    // node announced, so the key stays null the whole run.
    assert!(
        lxmf["propagation_node"].is_null(),
        "expected no propagation node on an isolated node, got {status}"
    );

    unsafe { resilum_node_stop(node) };
    unsafe { resilum_node_free(node) };
}

#[test]
fn lxmf_key_is_present_and_null_when_messaging_is_not_configured() {
    let dir = temp_dir();
    let node = unsafe { resilum_node_new_from_yaml(config("off", dir.path(), "").as_ptr()) };
    assert!(!node.is_null());
    assert_eq!(unsafe { resilum_node_start(node) }, RESILUM_OK);

    let status = status_json(node);
    assert!(status.as_object().unwrap().contains_key("lxmf"));
    assert!(status["lxmf"].is_null(), "expected null, got {status}");

    unsafe { resilum_node_stop(node) };
    unsafe { resilum_node_free(node) };
}
