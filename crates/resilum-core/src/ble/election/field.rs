use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::Notify;

use super::{Candidate, Facts};
use crate::ble::links::PeerId;

#[derive(Clone, Copy)]
struct Neighbour {
    can_host_at_all: bool,
    facts: Facts,
    spoke: bool,
    met_at_ms: u64,
}

#[derive(Clone, Default)]
pub struct Field {
    who: Arc<Mutex<HashMap<PeerId, Neighbour>>>,
    someone_new: Arc<Notify>,
}

impl Field {
    pub fn met_over_the_radio(&self, peer: PeerId, now_ms: u64) {
        self.lock().entry(peer).or_insert(Neighbour {
            can_host_at_all: false,
            facts: Facts::default(),
            spoke: false,
            met_at_ms: now_ms,
        });
        self.someone_new.notify_one();
    }

    pub async fn until_someone_new(&self) {
        self.someone_new.notified().await;
    }

    pub fn gone(&self, peer: PeerId) {
        self.lock().remove(&peer);
    }

    pub fn told_us(&self, peer: PeerId, facts: Facts, can_host_at_all: bool) {
        if let Some(neighbour) = self.lock().get_mut(&peer) {
            neighbour.can_host_at_all = can_host_at_all;
            neighbour.facts = facts;
            neighbour.spoke = true;
        }
    }

    #[must_use]
    pub fn whom_we_hear(&self) -> Vec<PeerId> {
        self.lock().keys().copied().collect()
    }

    #[must_use]
    pub fn how_many_we_hear(&self) -> u8 {
        u8::try_from(self.lock().len()).unwrap_or(u8::MAX)
    }

    #[must_use]
    pub fn someone_met_is_still_worth_hearing_out(&self, now_ms: u64, patience_ms: u64) -> bool {
        self.lock().values().any(|neighbour| {
            !neighbour.spoke && now_ms.saturating_sub(neighbour.met_at_ms) < patience_ms
        })
    }

    #[must_use]
    pub fn standing_with_us(&self, us: PeerId, ours: Facts, we_can_host: bool) -> Vec<Candidate> {
        let mut standing = self.standing();
        standing.push(Candidate::heard_on_the_air(us, ours, we_can_host));
        standing
    }

    #[must_use]
    pub fn standing(&self) -> Vec<Candidate> {
        self.lock()
            .iter()
            .map(|(peer, neighbour)| {
                Candidate::heard_on_the_air(*peer, neighbour.facts, neighbour.can_host_at_all)
            })
            .collect()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<PeerId, Neighbour>> {
        self.who
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests;
