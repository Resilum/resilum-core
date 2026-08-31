//! Serialization contract for the JSON snapshot — the consumer decodes this
//! shape, so a field renamed or dropped here is a silent breakage for it.

use resilum_core::coordinates::Coordinates;

use super::model::{BleStatus, CoordinatesStatus, LxmfStatus, NodeStatus};

fn bare_status() -> NodeStatus {
    NodeStatus {
        running: false,
        socks_port: Some(0),
        identity_hash: None,
        reachable_destinations: 0,
        interfaces: Vec::new(),
        transport: None,
        nostr_relays: Vec::new(),
        lxmf: None,
        tor: None,
        coordinates: CoordinatesStatus {
            ours: Coordinates::default().ours(),
            peers: Vec::new(),
        },
        links: Vec::new(),
        ble: BleStatus {
            hosting_the_group: false,
        },
    }
}

#[test]
fn a_node_keeping_no_links_serializes_them_as_an_array() {
    let json = serde_json::to_string(&bare_status()).expect("serialization");
    assert!(json.contains("\"links\":[]"), "JSON: {json}");
}

#[test]
fn empty_nostr_relays_serializes_as_array() {
    let json = serde_json::to_string(&bare_status()).expect("serialization");
    assert!(json.contains("\"nostr_relays\":[]"), "JSON: {}", json);
}

#[test]
fn a_node_that_has_placed_nobody_still_carries_its_own_coordinate() {
    let json = serde_json::to_string(&bare_status()).expect("serialization");

    assert!(json.contains("\"coordinates\":{\"ours\":{"), "JSON: {json}");
    assert!(json.contains("\"position\":["), "JSON: {json}");
    assert!(json.contains("\"peers\":[]"), "JSON: {json}");
}

#[test]
fn lxmf_off_serializes_as_null_key_not_absent() {
    let json = serde_json::to_string(&bare_status()).expect("serialization");
    assert!(json.contains("\"lxmf\":null"), "JSON: {}", json);
}

#[test]
fn tor_off_serializes_as_null_key_not_absent() {
    let json = serde_json::to_string(&bare_status()).expect("serialization");
    assert!(json.contains("\"tor\":null"), "JSON: {}", json);
}

#[test]
fn a_bootstrapped_tor_serializes_as_an_object() {
    let mut status = bare_status();
    status.tor = Some(super::model::TorStatus { bootstrapped: true });

    let json = serde_json::to_string(&status).expect("serialization");

    assert!(
        json.contains("\"tor\":{\"bootstrapped\":true}"),
        "JSON: {}",
        json
    );
}

#[test]
fn a_configured_lxmf_section_serializes_every_field_in_order() {
    let mut status = bare_status();
    status.lxmf = Some(LxmfStatus {
        ready: true,
        address: "a".repeat(32),
        queued_count: 1,
        queued_ids: vec!["c".repeat(64)],
        propagation_node: Some("b".repeat(32)),
    });
    let json = serde_json::to_string(&status).expect("serialization");
    let expected = format!(
        "\"lxmf\":{{\"ready\":true,\"address\":\"{}\",\"queued_count\":1,\"queued_ids\":[\"{}\"],\"propagation_node\":\"{}\"}}",
        "a".repeat(32),
        "c".repeat(64),
        "b".repeat(32),
    );
    assert!(json.contains(&expected), "JSON: {}", json);
}

/// An empty queue must serialize as an empty array, so the caller can iterate
/// it without a presence check.
#[test]
fn an_empty_queue_serializes_queued_ids_as_an_array() {
    let mut status = bare_status();
    status.lxmf = Some(LxmfStatus {
        ready: true,
        address: "a".repeat(32),
        queued_count: 0,
        queued_ids: Vec::new(),
        propagation_node: None,
    });
    let json = serde_json::to_string(&status).expect("serialization");
    assert!(json.contains("\"queued_ids\":[]"), "JSON: {}", json);
}

#[test]
fn no_selected_propagation_node_serializes_as_null_key_not_absent() {
    let mut status = bare_status();
    status.lxmf = Some(LxmfStatus {
        ready: true,
        address: "a".repeat(32),
        queued_count: 0,
        queued_ids: Vec::new(),
        propagation_node: None,
    });
    let json = serde_json::to_string(&status).expect("serialization");
    assert!(json.contains("\"propagation_node\":null"), "JSON: {}", json);
}
