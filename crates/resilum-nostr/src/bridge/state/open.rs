//! Where the bridge's two stores come from.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use resilum_core::lxmf::LxmfHandle;

use super::{InFlight, MeshSender, State};
use crate::bridge::recent::Recent;
use crate::bridge::retry::Schedule;
use crate::config::NostrConfig;
use crate::queue::Queue;
use crate::registry::Registry;
use crate::upstream::Upstream;

/// A gift wrap is one to two kilobytes on the wire, so this is the largest
/// backlog one subscriber can hold against the disk before it is refused.
const PER_SUBSCRIBER: usize = 256;

pub(in crate::bridge) struct Wiring {
    pub(in crate::bridge) mesh: MeshSender,
    pub(in crate::bridge) in_flight: InFlight,
    pub(in crate::bridge) lxmf: Arc<LxmfHandle>,
    pub(in crate::bridge) upstreams: Vec<Upstream>,
}

pub(in crate::bridge) fn open(
    cfg: NostrConfig,
    storage: Option<&Path>,
    wiring: Wiring,
) -> Result<State, String> {
    let (registry, queue) = stores(storage, cfg.retention)?;
    Ok(State {
        cfg,
        registry,
        queue,
        recent: Recent::default(),
        retry: Schedule::default(),
        upstreams: wiring.upstreams,
        lxmf: wiring.lxmf,
        in_flight: wiring.in_flight,
        mesh: wiring.mesh,
    })
}

/// A node given no storage path keeps everything in memory and loses it on
/// restart, which is what asking for no storage means. A node that was given
/// one is owed it: a directory the bridge cannot create, like a file it
/// cannot read back, would leave it running on an empty map and overwriting
/// what it never read, so it refuses to start at all.
fn stores(storage: Option<&Path>, retention: Duration) -> Result<(Registry, Queue), String> {
    let Some(root) = storage else {
        return Ok((
            Registry::ephemeral(retention),
            Queue::ephemeral(retention, PER_SUBSCRIBER),
        ));
    };
    let dir = root.join("nostr");
    resilum_core::storage::make_room_for(&dir).map_err(|e| {
        format!(
            "the nostr state directory at {} cannot be created: {e}",
            dir.display()
        )
    })?;
    Ok((
        Registry::open(dir.join("registry"), retention)?,
        Queue::open(dir.join("queue"), retention, PER_SUBSCRIBER)?,
    ))
}

#[cfg(test)]
mod tests;
