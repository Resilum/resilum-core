//! Start the Nostr bridge when the config asks for one.
//!
//! The `nostr:` section is invisible to `crate::config::load`: that goes
//! through `resilum_core::from_yaml`, which models only the keys `resilum_core`
//! knows and drops the rest silently.

use std::path::Path;
use std::sync::OnceLock;

use resilum_core::Node;
use resilum_core::discovery::Service;
use resilum_nostr::{BridgeHandle, NostrConfig, spawn};
use serde::Deserialize;

#[derive(Deserialize)]
struct NostrSection {
    #[serde(default)]
    nostr: Option<NostrConfig>,
}

/// `None` for an absent section — most nodes have none, that is not a failure —
/// or for a read or parse error, both logged so "why didn't my bridge start"
/// has an answer.
pub fn load(path: &Path) -> Option<NostrConfig> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) => {
            tracing::error!(error = %e, path = %path.display(), "nostr config read failed");
            return None;
        }
    };
    match serde_yaml_ng::from_str::<NostrSection>(&raw) {
        Ok(section) => section.nostr,
        Err(e) => {
            tracing::error!(error = %e, "nostr config parse failed");
            None
        }
    }
}

/// `None` when it fails to start — the daemon runs without it either way.
pub fn start(node: &Node, cfg: NostrConfig) -> Option<BridgeHandle> {
    let runtime = bridge_runtime()?;
    let _guard = runtime.enter();
    match spawn(node, cfg) {
        Ok(handle) => Some(handle),
        Err(e) => {
            tracing::error!(error = %e, "nostr bridge start failed");
            None
        }
    }
}

/// Announces this node's LXMF address as a Nostr relay, so peers can find the
/// bridge that `start` just brought up.
pub fn advertise_relay(node: &Node) {
    if let Some(identity) = node.identity() {
        node.advertise(
            Service::NOSTR_RELAY,
            resilum_core::identity::lxmf_address(identity),
        );
    }
}

/// A multi-thread runtime, built once and kept for the process: the bridge
/// spawns its tasks onto whichever runtime is current when it starts, and
/// the daemon otherwise never enters one.
fn bridge_runtime() -> Option<&'static tokio::runtime::Runtime> {
    static RUNTIME: OnceLock<Option<tokio::runtime::Runtime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .inspect_err(|e| tracing::error!(error = %e, "tokio runtime build failed"))
                .ok()
        })
        .as_ref()
}
