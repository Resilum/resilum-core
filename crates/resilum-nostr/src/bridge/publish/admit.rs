//! Who may publish through this bridge.
//!
//! Not the event's own signing key: under NIP-59 a gift wrap is signed by a
//! one-time key that says nothing about who wrote what it carries, so an
//! allow list read off it matches nothing an operator ever wrote down. The
//! mesh address the request arrived from is the only durable identity in
//! reach, and the registry is what ties it to an npub.

use crate::config::NostrConfig;
use crate::registry::Registry;

/// The mesh address a publish request arrived from, and how far this node
/// will vouch for it.
#[derive(Clone, Copy)]
pub(in crate::bridge) enum Source {
    Recalled([u8; 16]),
    Claimed([u8; 16]),
}

impl Source {
    pub(in crate::bridge) fn address(self) -> [u8; 16] {
        match self {
            Self::Recalled(address) | Self::Claimed(address) => address,
        }
    }
}

/// `Err` carries the reason, which reaches the sender in its ack.
pub(super) fn may_publish(
    cfg: &NostrConfig,
    registry: &Registry,
    source: Source,
) -> Result<(), String> {
    if cfg.allow_npubs.is_empty() {
        return Ok(());
    }
    // Only here: an open bridge has no allow list for a forged address to
    // walk past, but a populated one is exactly what an unproven address
    // must not slip through.
    let Source::Recalled(address) = source else {
        return Err("this node has not recalled that address's identity yet".into());
    };
    let registered = registry.pubkeys_at(&address);
    if registered.is_empty() {
        return Err("this bridge takes no subscription from that address".into());
    }
    // Any, not the first: a device holding two keys where one is listed is a
    // device the operator meant to admit, and map order is not a reason to
    // refuse it.
    if !registered.iter().any(|pubkey| cfg.may_use(pubkey)) {
        return Err("this bridge does not carry that npub".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
