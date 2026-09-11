use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use leviculum_std::InterfaceId;

use crate::coordinates::{Coordinates, PeerId};
use crate::discovery::quota;

pub type DetachesWhenDropped = Box<dyn std::any::Any + Send>;

pub(crate) struct Attached {
    pub(crate) service: String,
    pub(crate) announced_by: Option<PeerId>,
    pub(crate) interface: InterfaceId,
    pub(crate) _detaches_when_dropped: DetachesWhenDropped,
}

pub struct Attachments {
    held: Mutex<HashMap<String, Attached>>,
    coordinates: Arc<Coordinates>,
}

pub struct Link {
    pub peer: PeerId,
    pub transport: String,
    pub interface: InterfaceId,
}

impl Attachments {
    #[must_use]
    pub fn new(coordinates: Arc<Coordinates>) -> Self {
        Self {
            held: Mutex::new(HashMap::new()),
            coordinates,
        }
    }

    pub(crate) fn estimate_of(&self, peer: &PeerId) -> Option<Duration> {
        self.coordinates.estimated_rtt(peer)
    }

    pub(crate) fn holds(&self, attached_as: &str) -> bool {
        self.lock().contains_key(attached_as)
    }

    pub(crate) fn hold(&self, attached_as: String, mut attached: Attached) {
        let mut held = self.lock();
        if attached.announced_by.is_none() {
            attached.announced_by = held.get(&attached_as).and_then(|held| held.announced_by);
        }
        held.insert(attached_as, attached);
    }

    pub(crate) fn learn_who_announced(&self, attached_as: &str, peer: PeerId) -> bool {
        let mut held = self.lock();
        let Some(anonymous) = held
            .get_mut(attached_as)
            .filter(|held| held.announced_by.is_none())
        else {
            return false;
        };
        anonymous.announced_by = Some(peer);
        true
    }

    pub(crate) fn release(&self, attached_as: &str) -> Option<Attached> {
        self.lock().remove(attached_as)
    }

    pub(crate) fn release_service(&self, service: &str) {
        self.lock().retain(|_, held| held.service != service);
    }

    #[must_use]
    pub fn whose_links_we_keep(&self) -> Vec<PeerId> {
        let mut kept: Vec<PeerId> = self
            .lock()
            .values()
            .filter_map(|held| held.announced_by)
            .collect();
        kept.sort_unstable();
        kept.dedup();
        kept
    }

    #[must_use]
    pub fn links(&self) -> Vec<Link> {
        self.lock()
            .values()
            .filter_map(|held| {
                Some(Link {
                    peer: held.announced_by?,
                    transport: held.service.clone(),
                    interface: held.interface,
                })
            })
            .collect()
    }

    pub(crate) fn kept(&self) -> Vec<quota::Peer> {
        self.lock()
            .iter()
            .map(|(attached_as, held)| quota::Peer {
                attached_as: attached_as.clone(),
                estimate: held
                    .announced_by
                    .and_then(|peer| self.coordinates.estimated_rtt(&peer)),
                node: held.announced_by,
                reached: quota::Reached::attached_as(&held.service),
            })
            .collect()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, Attached>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod tests;
