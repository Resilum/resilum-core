//! LXMF messaging: the router runs inside the engine's tick, the app talks to
//! it through an [`LxmfHandle`], and [`send`]/[`poll`] map the FFI's JSON onto
//! `leviculum_lxmf` types.

mod checkpoint;
mod handle;
mod inbox;
pub mod poll;
mod processor;
pub mod send;
mod stamp;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use leviculum_lxmf::Field;
use leviculum_lxmf::constants::{FIELD_CUSTOM_DATA, FIELD_CUSTOM_TYPE};
use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNodeBuilder;
use serde_json::Value;

pub use handle::LxmfHandle;

use crate::config::LxmfConfig;

/// Takes the builder rather than a built node: a processor installed later
/// could hold a handle to the node it runs inside, and calling one of that
/// handle's methods from a hook deadlocks the core on the first event.
pub(crate) fn install(
    builder: ReticulumNodeBuilder,
    config: &LxmfConfig,
    identity: &Identity,
    storage_path: Option<&Path>,
) -> (ReticulumNodeBuilder, LxmfHandle) {
    let registered = Arc::new(AtomicBool::new(false));
    let address = crate::identity::lxmf_address_hex(identity);
    let inbox = Arc::new(match storage_path {
        Some(dir) => inbox::Inbox::open(dir.join(INBOX_FILE)),
        None => inbox::Inbox::ephemeral(),
    });
    let (handle, commands, events) = handle::channel(address, registered.clone(), inbox.clone());
    let (stamp_tx, stamp_rx) = tokio::sync::mpsc::unbounded_channel();
    stamp::spawn(stamp_rx, handle.sender());
    // No storage directory means no durable queue: a restart loses whatever
    // was still in flight.
    let checkpoint =
        storage_path.map(|dir| checkpoint::Checkpoint::open(dir.join(CHECKPOINT_FILE)));
    let processor = processor::LxmfProcessor::new(
        config.clone(),
        identity.clone(),
        processor::Wiring {
            commands,
            events,
            inbox,
            stamps: stamp_tx,
            registered,
            checkpoint,
        },
    );
    (builder.core_processor(processor), handle)
}

/// Where the router's checkpoint lives, under the node's storage directory.
const CHECKPOINT_FILE: &str = "lxmf_state";

/// Received messages the app has not taken yet.
const INBOX_FILE: &str = "lxmf_inbox";

/// The app's `{custom_type, custom_data}` as LXMF custom fields — each value a
/// single msgpack value, as `Message::create` and the wire require.
fn encode_fields(custom_type: Option<&str>, custom_data: Option<&Value>) -> Vec<Field> {
    let mut fields = Vec::new();
    if let Some(t) = custom_type
        && let Ok(bytes) = rmp_serde::to_vec(t)
    {
        fields.push((FIELD_CUSTOM_TYPE, bytes));
    }
    if let Some(d) = custom_data
        && let Ok(bytes) = rmp_serde::to_vec(d)
    {
        fields.push((FIELD_CUSTOM_DATA, bytes));
    }
    fields
}

/// Inverse of [`encode_fields`]: recover `{custom_type, custom_data}` from an
/// LXMF message's fields, ignoring fields the app doesn't model.
fn decode_fields(fields: &[Field]) -> (Option<String>, Option<Value>) {
    let mut custom_type = None;
    let mut custom_data = None;
    for (id, bytes) in fields {
        match *id {
            FIELD_CUSTOM_TYPE => custom_type = rmp_serde::from_slice(bytes).ok(),
            FIELD_CUSTOM_DATA => custom_data = rmp_serde::from_slice(bytes).ok(),
            _ => {}
        }
    }
    (custom_type, custom_data)
}

#[cfg(test)]
mod tests;
