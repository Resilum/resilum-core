//! Node state snapshot for the UI, as a JSON string over the C ABI.
//!
//! JSON keeps the wire schema evolvable without touching the C ABI: new fields
//! appear in the object, the two entry points below stay fixed. It never leaves
//! the device — the app decodes it locally.

mod model;

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;

use model::{Interface, NodeStatus, Transport, hex16, interface_source};

use crate::guard;
use crate::node::ResilumNode;

/// A JSON snapshot of node state (running, socks_port, identity, path_count,
/// interfaces, transport counters), or null on error. Free the returned string
/// with `resilum_string_free`.
///
/// # Safety
/// `node` must be a live handle from `resilum_node_new_*` or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_node_status_json(node: *const ResilumNode) -> *mut c_char {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            return std::ptr::null_mut();
        };
        let mut status = NodeStatus {
            running: node.0.is_running(),
            socks_port: node.0.socks_port(),
            identity: None,
            path_count: 0,
            interfaces: Vec::new(),
            transport: None,
        };
        if let Some(engine) = node.0.engine() {
            status.identity = Some(hex16(&engine.identity_hash()));
            status.path_count = engine.path_count();
            let mut peers: HashMap<usize, Vec<String>> = HashMap::new();
            let mut peer_nodes: HashMap<usize, Vec<String>> = HashMap::new();
            for p in engine.path_table_entries() {
                if p.hops == 1 {
                    peers
                        .entry(p.interface_index)
                        .or_default()
                        .push(hex16(&p.hash));
                    if let Some(id) = engine.get_identity(&p.hash.into()) {
                        let node = hex16(id.hash());
                        let nodes = peer_nodes.entry(p.interface_index).or_default();
                        if !nodes.contains(&node) {
                            nodes.push(node);
                        }
                    }
                }
            }
            status.interfaces = engine
                .interface_stats()
                .into_iter()
                .map(|i| Interface {
                    source: interface_source(&i.name),
                    kind: i.kind.as_str(),
                    discovered_via: node
                        .0
                        .discovered_via(i.interface_id)
                        .unwrap_or_else(|| "direct".into()),
                    peer_nodes: peer_nodes
                        .get(&i.interface_id.0)
                        .cloned()
                        .unwrap_or_default(),
                    peer_hashes: peers.get(&i.interface_id.0).cloned().unwrap_or_default(),
                    name: i.name,
                    online: i.online,
                    local_client: i.is_local_client,
                    rx_bytes: i.rx_bytes,
                    tx_bytes: i.tx_bytes,
                    bitrate: i.configured_bitrate,
                })
                .collect();
            let t = engine.transport_stats();
            status.transport = Some(Transport {
                packets_sent: t.packets_sent(),
                packets_received: t.packets_received(),
                packets_forwarded: t.packets_forwarded(),
                packets_dropped: t.packets_dropped(),
                announces_processed: t.announces_processed(),
            });
        }
        match serde_json::to_string(&status)
            .ok()
            .and_then(|s| CString::new(s).ok())
        {
            Some(c) => c.into_raw(),
            None => std::ptr::null_mut(),
        }
    })
}

/// Free a string returned by this library (e.g. `resilum_node_status_json`).
///
/// # Safety
/// `s` must be a pointer returned by this library, or null. Do not free twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}
