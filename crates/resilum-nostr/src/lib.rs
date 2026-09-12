//! A Nostr proxy over LXMF: publish a signed event to the Nostr network, and
//! route events matching a subscriber's filters back onto the mesh.
//!
//! Nothing here signs, re-encrypts or inspects content — an event arrives
//! signed by its sender and, under NIP-17, encrypted to its recipient. Direct
//! messaging is one filter; a feed later is another, not another crate.
//!
//! Everything the bridge is built from stays inside, so the errors those
//! parts hand each other are not a contract with anyone outside.

mod bridge;
mod config;
mod event;
mod npub;
mod queue;
mod registry;
#[cfg(test)]
mod signed;
mod subscription;
mod upstream;

pub use bridge::{BridgeHandle, StartError, spawn};
pub use config::NostrConfig;
