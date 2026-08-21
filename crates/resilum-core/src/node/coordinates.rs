use super::Node;
use crate::coordinates::{Claimed, Placed};

impl Node {
    #[must_use]
    pub fn own_coordinate(&self) -> Claimed {
        self.coordinates.ours()
    }

    #[must_use]
    pub fn placed_peers(&self) -> Vec<Placed> {
        self.coordinates.placed()
    }
}
