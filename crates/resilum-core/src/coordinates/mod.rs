//! Where this node and its peers sit in latency space.

mod claimed;
pub mod exchange;
mod peer;
mod window;

pub use claimed::Claimed;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use violin::heapless::VecD;
use violin::{Coord, Node};

use peer::Peer;

pub type PeerId = [u8; 16];
pub type LinkId = usize;

type Space = VecD<3>;
type Adjustments = VecD<8>;

pub struct Coordinates {
    ours: Mutex<Node<Space, Adjustments>>,
    peers: Mutex<BTreeMap<PeerId, Peer>>,
}

impl Default for Coordinates {
    fn default() -> Self {
        Self {
            ours: Mutex::new(Node::new()),
            peers: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Coordinates {
    #[must_use]
    pub fn ours(&self) -> Claimed {
        Claimed::of(self.node().coordinate())
    }

    pub fn heard(&self, peer: PeerId, theirs: Claimed, now: f64) -> bool {
        if theirs.believable().is_none() {
            return false;
        }
        let mut peers = self.peers();
        let entry = peers.entry(peer).or_default();
        entry.claimed = Some(theirs);
        entry.last_heard = now;
        true
    }

    pub fn believe(
        &self,
        peer: PeerId,
        over: LinkId,
        rtt: Duration,
        theirs: Claimed,
        now: f64,
    ) -> bool {
        let Some(remote) = theirs.believable() else {
            return false;
        };
        self.heard(peer, theirs, now);
        let mut peers = self.peers();
        let entry = peers.entry(peer).or_default();
        entry.measured(over, rtt);
        let Some(fastest) = entry.fastest_link() else {
            return true;
        };
        drop(peers);
        self.pull_towards(&remote, fastest);
        true
    }

    #[must_use]
    pub fn estimated_rtt(&self, peer: &PeerId) -> Option<Duration> {
        let theirs = self.peers().get(peer)?.claimed?.believable()?;
        Some(self.node().distance_to(&theirs))
    }

    #[must_use]
    pub fn nearest(&self, count: usize) -> Vec<(PeerId, Duration)> {
        let claimed: Vec<(PeerId, Claimed)> = self
            .peers()
            .iter()
            .filter_map(|(peer, held)| Some((*peer, held.claimed?)))
            .collect();
        let node = self.node();
        let mut placed: Vec<(PeerId, Duration)> = claimed
            .into_iter()
            .filter_map(|(peer, claim)| Some((peer, node.distance_to(&claim.believable()?))))
            .collect();
        placed.sort_by_key(|(_, estimate)| *estimate);
        placed.truncate(count);
        placed
    }

    pub fn forget_link(&self, over: LinkId) {
        for peer in self.peers().values_mut() {
            peer.forget_link(over);
        }
    }

    pub fn forget_before(&self, cutoff: f64) {
        self.peers().retain(|_, peer| peer.last_heard >= cutoff);
    }

    fn pull_towards(&self, remote: &Coord<Space>, rtt: Duration) {
        let mut node = self.node();
        let before = Claimed::of(node.coordinate());
        let moved = node.try_update(rtt, remote).is_ok();
        if moved && Claimed::of(node.coordinate()).believable().is_some() {
            node.update_gravity(&Coord::default());
            return;
        }
        node.set_coordinate(before.believable().unwrap_or_default());
    }

    fn node(&self) -> std::sync::MutexGuard<'_, Node<Space, Adjustments>> {
        self.ours.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn peers(&self) -> std::sync::MutexGuard<'_, BTreeMap<PeerId, Peer>> {
        self.peers.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests;
