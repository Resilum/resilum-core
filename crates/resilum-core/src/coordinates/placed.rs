use std::time::Duration;

use super::{Claimed, Coordinates, PeerId};

pub struct Placed {
    pub peer: PeerId,
    pub at: Claimed,
    pub estimated_rtt: Duration,
}

impl Coordinates {
    #[must_use]
    pub fn estimated_rtt(&self, peer: &PeerId) -> Option<Duration> {
        let theirs = self.peers().get(peer)?.claimed?.believable()?;
        Some(self.node().distance_to(&theirs))
    }

    #[must_use]
    pub fn placed(&self) -> Vec<Placed> {
        let claimed: Vec<(PeerId, Claimed)> = self
            .peers()
            .iter()
            .filter_map(|(peer, held)| Some((*peer, held.claimed?)))
            .collect();
        let node = self.node();
        let mut placed: Vec<Placed> = claimed
            .into_iter()
            .filter_map(|(peer, at)| {
                Some(Placed {
                    peer,
                    at,
                    estimated_rtt: node.distance_to(&at.believable()?),
                })
            })
            .collect();
        placed.sort_by_key(|placed| placed.estimated_rtt);
        placed
    }

    #[must_use]
    pub fn nearest(&self, count: usize) -> Vec<Placed> {
        let mut nearest = self.placed();
        nearest.truncate(count);
        nearest
    }
}
