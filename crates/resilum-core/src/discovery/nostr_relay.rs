//! Discovery of a Nostr bridge: the id a device needs to reach one without
//! being told it by hand, and the address book of ones it has already heard
//! about on the mesh.

use std::sync::{Arc, Mutex};

use super::DiscoveryPlugin;

/// Announces are unauthenticated, so nothing but this cap stops a chatty mesh
/// from growing the set forever; the oldest entry gives way to the newest.
const MAX_DISCOVERED: usize = 32;

pub struct NostrRelayPlugin {
    advertise: Mutex<Option<[u8; 16]>>,
    discovered: Mutex<Vec<[u8; 16]>>,
}

impl NostrRelayPlugin {
    #[must_use]
    pub fn new(advertise: Option<[u8; 16]>) -> Arc<Self> {
        Arc::new(Self {
            advertise: Mutex::new(advertise),
            discovered: Mutex::new(Vec::new()),
        })
    }

    /// Bridge addresses heard from peers' announces, oldest first.
    #[must_use]
    pub fn discovered(&self) -> Vec<[u8; 16]> {
        self.discovered
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Start or stop announcing this node's own address. A bridge calls this
    /// once it knows the node's LXMF address and whether it is publishing;
    /// `None` withdraws the announce without touching what was discovered.
    pub fn set_advertise(&self, advertise: Option<[u8; 16]>) {
        *self.advertise.lock().unwrap_or_else(|e| e.into_inner()) = advertise;
    }
}

impl DiscoveryPlugin for NostrRelayPlugin {
    /// `None` unless this node was handed its own address to publish — a
    /// personal bridge stays silent while it keeps listening.
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        self.advertise
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .map(|addr| addr.to_vec())
    }

    fn consume_endpoint(&self, payload: &[u8], _announcer_pubkey: Option<&[u8]>) {
        let Ok(addr) = <[u8; 16]>::try_from(payload) else {
            return;
        };
        let mut discovered = self.discovered.lock().unwrap_or_else(|e| e.into_inner());
        if discovered.contains(&addr) {
            return;
        }
        if discovered.len() >= MAX_DISCOVERED {
            discovered.remove(0);
        }
        discovered.push(addr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A personal bridge must be reachable by the people who know it
    /// without announcing itself to everyone who hears the mesh.
    #[test]
    fn a_bridge_that_does_not_publish_still_learns_of_others() {
        let plugin = NostrRelayPlugin::new(None);
        plugin.consume_endpoint(&[0xab; 16], None);

        assert_eq!(plugin.produce_endpoint(), None);
        assert_eq!(plugin.discovered(), vec![[0xab; 16]]);
    }

    #[test]
    fn an_endpoint_of_the_wrong_length_is_ignored() {
        let plugin = NostrRelayPlugin::new(Some([0x11; 16]));
        plugin.consume_endpoint(&[0xab; 8], None);

        assert_eq!(plugin.produce_endpoint(), Some(vec![0x11; 16]));
        assert!(plugin.discovered().is_empty());
    }

    /// Proves the cap actually evicts rather than merely stopping growth: the
    /// very first address heard must be gone once a 33rd arrives.
    #[test]
    fn the_discovered_set_evicts_the_oldest_once_it_is_full() {
        let plugin = NostrRelayPlugin::new(None);
        for i in 0..=MAX_DISCOVERED {
            plugin.consume_endpoint(&[i as u8; 16], None);
        }

        let discovered = plugin.discovered();
        assert_eq!(discovered.len(), MAX_DISCOVERED);
        assert!(!discovered.contains(&[0u8; 16]));
        assert!(discovered.contains(&[MAX_DISCOVERED as u8; 16]));
    }

    /// A bridge that turns publishing off must stop announcing, not just
    /// start with nothing to announce.
    #[test]
    fn a_plugin_announces_only_while_its_owner_is_publishing() {
        let plugin = NostrRelayPlugin::new(None);
        assert_eq!(plugin.produce_endpoint(), None);

        plugin.set_advertise(Some([0x22; 16]));
        assert_eq!(plugin.produce_endpoint(), Some(vec![0x22; 16]));

        plugin.set_advertise(None);
        assert_eq!(plugin.produce_endpoint(), None);
    }
}
