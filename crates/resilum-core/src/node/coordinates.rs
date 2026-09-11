use super::Node;
use crate::coordinates::{Claimed, Placed};
use crate::discovery::Link;

impl Node {
    #[must_use]
    pub fn own_coordinate(&self) -> Claimed {
        self.coordinates.ours()
    }

    #[must_use]
    pub fn placed_peers(&self) -> Vec<Placed> {
        self.coordinates.placed()
    }

    #[must_use]
    pub fn links_we_keep(&self) -> Vec<Link> {
        self.attachments.links()
    }

    #[must_use]
    pub fn estimated_rtt(&self, peer: &crate::coordinates::PeerId) -> Option<std::time::Duration> {
        self.coordinates.estimated_rtt(peer)
    }

    #[must_use]
    pub fn measured_rtt_over(
        &self,
        peer: &crate::coordinates::PeerId,
        over: crate::coordinates::LinkId,
    ) -> Option<std::time::Duration> {
        self.coordinates.measured_over(peer, over)
    }
}
