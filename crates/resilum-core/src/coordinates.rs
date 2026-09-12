//! Where this node and its peers sit in latency space.

mod beginning;
mod claimed;
pub mod exchange;
mod peer;
mod placed;
mod window;

pub use claimed::Claimed;
pub use placed::Placed;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use violin::heapless::VecD;
use violin::{Coord, Node};

use beginning::{knowing_nothing_of_where_we_are, without_a_runaway_last_mile};

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
            ours: Mutex::new(knowing_nothing_of_where_we_are()),
            peers: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Coordinates {
    #[must_use]
    pub fn ours(&self) -> Claimed {
        Claimed::of(self.node().coordinate())
    }

    #[must_use]
    pub fn how_wrong_we_are(&self) -> f64 {
        self.node().error_estimate()
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
        entry.measured(over, rtt, now);
        let Some(fastest) = entry.fastest_link() else {
            return true;
        };
        drop(peers);
        self.pull_towards(&remote, fastest);
        true
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
        if node.try_update(rtt, remote).is_ok() {
            node.update_gravity(&Coord::default());
            if Claimed::of(node.coordinate()).believable().is_some() {
                let trimmed = without_a_runaway_last_mile(&node);
                node.set_coordinate(trimmed);
                return;
            }
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
