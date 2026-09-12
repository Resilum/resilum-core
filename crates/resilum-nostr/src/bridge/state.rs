//! What every part of the loop reaches for: the two stores, the relay
//! handles, and the one way onto the mesh.

mod frames;
mod open;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use resilum_core::lxmf::LxmfHandle;
use tokio_tungstenite::tungstenite::protocol::frame::Utf8Bytes;

use super::recent::Recent;
use super::relay::Stores;
use super::retry::Schedule;
use super::tie::Tie;
use crate::config::NostrConfig;
use crate::queue::Queue;
use crate::registry::{BatchId, Registry};
use crate::upstream::Upstream;

pub(super) use open::{Wiring, open};

/// Builds, signs and submits one message, recording its `Tie` before the
/// message can be delivered. Boxed because the node identity it signs with
/// is not a type this crate can name.
pub(super) type MeshSender = Box<dyn Fn(&str, Option<Tie>) -> Result<(), String> + Send + Sync>;

/// LXMF message id (hex) to the entry it carries, and when it went out.
pub(super) type InFlight = Arc<Mutex<HashMap<String, (Tie, Instant)>>>;

pub(super) struct State {
    pub(super) cfg: NostrConfig,
    pub(super) registry: Registry,
    pub(super) queue: Queue,
    pub(super) recent: Recent,
    pub(super) retry: Schedule,
    pub(super) upstreams: Vec<Upstream>,
    pub(super) lxmf: Arc<LxmfHandle>,
    in_flight: InFlight,
    mesh: MeshSender,
}

impl State {
    pub(super) fn submit(&self, json: &str, tie: Option<Tie>) -> Result<(), String> {
        (self.mesh)(json, tie)
    }

    /// Everything the routing decision reads, and nothing it does not.
    pub(super) fn stores(&self) -> Stores<'_> {
        Stores {
            cfg: &self.cfg,
            registry: &self.registry,
            queue: &self.queue,
            recent: &self.recent,
        }
    }

    /// Returns how many relays the frame went to.
    pub(super) fn broadcast(&self, frame: &str) -> usize {
        // Every relay is sent the same bytes, and a publish frame carries a
        // whole event, so the frame is shared rather than copied per relay.
        let frame = Utf8Bytes::from(frame);
        let mut sent = 0;
        for up in self.upstreams.iter().filter(|up| up.is_up()) {
            up.send(frame.clone());
            sent += 1;
        }
        sent
    }

    /// Every batch currently populated — what a freshly connected upstream
    /// needs, since it starts out asking for nothing at all.
    pub(super) fn request_frames(&self) -> Vec<String> {
        frames::requests(&self.cfg.inbound_kinds(), &self.registry.live_marks(now()))
    }

    /// One batch's frame, reissued because its membership just changed.
    /// Returns how many relays it went to.
    pub(super) fn broadcast_batch(&self, batch: BatchId) -> usize {
        let marks: Vec<frames::Mark> = self
            .registry
            .live_marks(now())
            .into_iter()
            .filter_map(|mark| (mark.batch == batch).then_some((mark.pubkey, mark.last_seen)))
            .collect();
        self.broadcast(&frames::batch_frame(
            &self.cfg.inbound_kinds(),
            batch,
            &marks,
        ))
    }

    pub(super) fn delivered(&self, message_id: &str) {
        if let Some(tie) = self.forget(message_id) {
            self.queue.resolve(&tie.event_id, &tie.subscriber);
        }
    }

    /// Any other terminal state leaves the entry queued; forgetting the id
    /// is what lets the next maintenance tick send it again.
    pub(super) fn not_delivered(&self, message_id: &str) {
        self.forget(message_id);
    }

    /// The entries LXMF has taken and not yet reported on.
    pub(super) fn awaiting_report(&self) -> HashSet<Tie> {
        self.ties().values().map(|(tie, _)| *tie).collect()
    }

    /// A message LXMF never reported on at all would otherwise suppress its
    /// own retry forever.
    pub(super) fn forget_stale_ties(&self, after: std::time::Duration) {
        self.ties().retain(|_, (_, at)| at.elapsed() < after);
    }

    fn forget(&self, message_id: &str) -> Option<Tie> {
        self.ties().remove(message_id).map(|(tie, _)| tie)
    }

    fn ties(&self) -> std::sync::MutexGuard<'_, HashMap<String, (Tie, Instant)>> {
        self.in_flight.lock().unwrap_or_else(|e| e.into_inner())
    }
}

pub(super) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}
