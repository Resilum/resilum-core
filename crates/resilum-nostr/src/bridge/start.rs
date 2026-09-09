//! Starting the bridge, and stopping it when the handle goes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use data_encoding::HEXLOWER;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::run;
use super::state::{self, InFlight, MeshSender, Wiring};
use crate::config::NostrConfig;
use crate::upstream::{Deadlines, Upstream, UpstreamRunner, proto};

/// Why the bridge did not start.
#[derive(Debug)]
#[non_exhaustive]
pub enum StartError {
    NoRuntime,
    LxmfDisabled,
    NodeNotRunning,
    /// The storage the node was configured with cannot be kept state under.
    Storage(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRuntime => f.write_str("the bridge needs a tokio runtime"),
            Self::LxmfDisabled => f.write_str("lxmf messaging is not enabled"),
            Self::NodeNotRunning => f.write_str("the node is not running"),
            Self::Storage(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for StartError {}

/// Holds the bridge open. Dropping it stops every task the bridge started;
/// nothing else keeps them alive.
#[must_use = "the bridge runs only for as long as this handle is held"]
pub struct BridgeHandle {
    tasks: Vec<JoinHandle<()>>,
}

impl Drop for BridgeHandle {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// Runs the bridge on the current tokio runtime until the returned handle is
/// dropped.
pub fn spawn(node: &resilum_core::Node, cfg: NostrConfig) -> Result<BridgeHandle, StartError> {
    let runtime = Handle::try_current().map_err(|_| StartError::NoRuntime)?;
    let lxmf = Arc::clone(node.lxmf().ok_or(StartError::LxmfDisabled)?);
    let identity = node.identity().ok_or(StartError::NodeNotRunning)?.clone();
    let in_flight: InFlight = Arc::new(Mutex::new(HashMap::new()));
    let submitter = Arc::clone(&lxmf);
    let ties = Arc::clone(&in_flight);
    let mesh: MeshSender = Box::new(move |json, tie| {
        let source = resilum_core::identity::lxmf_address(&identity);
        let message = resilum_core::lxmf::send::build_message(json, &identity, source, now_secs())
            .map_err(|e| e.to_string())?;
        // Before the submission: a delivery report for a message nothing is
        // waiting on leaves its entry queued until retention, and sent again
        // in the meantime.
        if let Some(tie) = tie {
            let mut held = ties.lock().unwrap_or_else(|e| e.into_inner());
            held.insert(HEXLOWER.encode(&message.message_id), (tie, Instant::now()));
        }
        submitter.submit(message).map_err(|e| e.to_string())
    });

    let (incoming, relays) = mpsc::unbounded_channel();
    let (upstreams, runners) = upstream_pairs(&cfg.upstreams, &incoming);
    // Every remaining sender belongs to an `UpstreamRunner`, so the loop's
    // receiver ends when the last of them does — at once with none at all.
    drop(incoming);
    let state = Arc::new(
        state::open(
            cfg,
            node.config().storage_path.as_deref(),
            Wiring {
                mesh,
                in_flight,
                lxmf,
                upstreams,
            },
        )
        .map_err(StartError::Storage)?,
    );

    let mut tasks: Vec<JoinHandle<()>> = runners
        .into_iter()
        .map(|runner| {
            let state = Arc::clone(&state);
            runtime.spawn(runner.run(move || state.request_frames()))
        })
        .collect();
    tracing::info!(
        address = state.lxmf.address_hex(),
        upstreams = state.upstreams.len(),
        "the nostr bridge is running"
    );
    tasks.push(runtime.spawn(run::run(state, relays)));
    Ok(BridgeHandle { tasks })
}

fn upstream_pairs(
    urls: &[String],
    incoming: &mpsc::UnboundedSender<proto::Incoming>,
) -> (Vec<Upstream>, Vec<UpstreamRunner>) {
    urls.iter()
        .map(|url| Upstream::pair(url.clone(), incoming.clone(), Deadlines::default()))
        .unzip()
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or_default()
}
