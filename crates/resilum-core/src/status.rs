//! What this node can say about itself, in one shape for every consumer.

pub use self::build::snapshot;
pub use self::model::{
    BleStatus, CoordinatesStatus, Interface, Link, LxmfStatus, NodeStatus, PlacedPeer, TorStatus,
    Transport,
};

mod build;
mod lxmf;
mod model;
#[cfg(test)]
mod tests;
