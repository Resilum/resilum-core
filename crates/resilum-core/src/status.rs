//! What this node can say about itself, in one shape for every consumer.

mod build;
mod lxmf;
mod model;

pub use build::snapshot;
pub use model::{
    BleStatus, CoordinatesStatus, Interface, Link, LxmfStatus, NodeStatus, PlacedPeer, TorStatus,
    Transport,
};

#[cfg(test)]
mod tests;
