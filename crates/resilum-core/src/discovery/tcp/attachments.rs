use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use leviculum_std::interfaces::TcpClientHandle;

use super::quota;
use crate::coordinates::{Coordinates, PeerId};

pub(super) struct Attached {
    pub(super) service: String,
    pub(super) announced_by: Option<PeerId>,
    pub(super) handle: TcpClientHandle,
}

pub struct Attachments {
    held: Mutex<HashMap<String, Attached>>,
    coordinates: Arc<Coordinates>,
}

impl Attachments {
    #[must_use]
    pub fn new(coordinates: Arc<Coordinates>) -> Self {
        Self {
            held: Mutex::new(HashMap::new()),
            coordinates,
        }
    }

    pub(super) fn estimate_of(&self, peer: &PeerId) -> Option<Duration> {
        self.coordinates.estimated_rtt(peer)
    }

    pub(super) fn holds(&self, attached_as: &str) -> bool {
        self.lock().contains_key(attached_as)
    }

    pub(super) fn hold(&self, attached_as: String, attached: Attached) {
        self.lock().insert(attached_as, attached);
    }

    pub(super) fn release(&self, attached_as: &str) -> Option<Attached> {
        self.lock().remove(attached_as)
    }

    pub(super) fn release_service(&self, service: &str) {
        self.lock().retain(|_, held| held.service != service);
    }

    pub(super) fn kept(&self) -> Vec<quota::Peer> {
        self.lock()
            .iter()
            .map(|(attached_as, held)| quota::Peer {
                attached_as: attached_as.clone(),
                estimate: held
                    .announced_by
                    .and_then(|peer| self.coordinates.estimated_rtt(&peer)),
            })
            .collect()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, Attached>> {
        self.held.lock().unwrap_or_else(|e| e.into_inner())
    }
}
