//! Exercise the C ABI from Rust: lifecycle and error reporting.

use std::ffi::{CStr, CString};

use resilum_ffi::*;

#[test]
fn lifecycle_via_ffi() {
    let yaml =
        CString::new("instance_name: ffi-test\ndefault_anchors: false\ndiscover: false\n").unwrap();
    let node = unsafe { resilum_node_new_from_yaml(yaml.as_ptr()) };
    assert!(!node.is_null());

    assert_eq!(unsafe { resilum_node_start(node) }, RESILUM_OK);
    assert_eq!(unsafe { resilum_node_is_running(node) }, 1);

    let event = unsafe { resilum_node_poll_event(node) };
    assert!(!event.is_null());
    assert_eq!(unsafe { resilum_event_kind(event) }, RESILUM_EVENT_STARTED);
    let mut len = 99usize;
    assert!(unsafe { resilum_event_source(event, &mut len) }.is_null());
    assert_eq!(len, 0);
    unsafe { resilum_event_free(event) };

    assert_eq!(unsafe { resilum_node_stop(node) }, RESILUM_OK);
    assert_eq!(unsafe { resilum_node_is_running(node) }, 0);

    unsafe { resilum_node_free(node) };
}

#[test]
fn malformed_config_reports_an_error() {
    let yaml = CString::new("not: [valid").unwrap();
    let node = unsafe { resilum_node_new_from_yaml(yaml.as_ptr()) };
    assert!(node.is_null());

    let err = resilum_last_error();
    assert!(!err.is_null());
    let msg = unsafe { CStr::from_ptr(err) }.to_str().unwrap();
    assert!(msg.contains("parse config"), "unexpected error: {msg}");
}

#[test]
fn null_config_is_rejected() {
    assert!(unsafe { resilum_node_new_from_yaml(std::ptr::null()) }.is_null());
}
