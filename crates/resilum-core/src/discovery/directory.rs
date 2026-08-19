//! The address book of peers offering a service at a mesh address, and this
//! node's own entry in it.

use std::sync::Mutex;

use super::DiscoveryPlugin;

/// Announces are unauthenticated, so nothing but this cap stops a chatty mesh
/// from growing the set forever; the oldest entry gives way to the newest.
const MAX_DISCOVERED: usize = 32;

#[derive(Default)]
pub struct ServiceDirectory {
    announced: Mutex<Option<[u8; 16]>>,
    discovered: Mutex<Vec<[u8; 16]>>,
}

impl ServiceDirectory {
    /// Addresses heard from peers' announces, oldest first.
    #[must_use]
    pub fn discovered(&self) -> Vec<[u8; 16]> {
        self.discovered
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn announce(&self, address: [u8; 16]) {
        *self.announced.lock().unwrap_or_else(|e| e.into_inner()) = Some(address);
    }

    pub fn withdraw(&self) {
        *self.announced.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

impl DiscoveryPlugin for ServiceDirectory {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        self.announced
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

    #[test]
    fn a_node_that_does_not_announce_still_learns_of_others() {
        let directory = ServiceDirectory::default();
        directory.consume_endpoint(&[0xab; 16], None);

        assert_eq!(directory.produce_endpoint(), None);
        assert_eq!(directory.discovered(), vec![[0xab; 16]]);
    }

    #[test]
    fn an_endpoint_of_the_wrong_length_is_ignored() {
        let directory = ServiceDirectory::default();
        directory.announce([0x11; 16]);
        directory.consume_endpoint(&[0xab; 8], None);

        assert_eq!(directory.produce_endpoint(), Some(vec![0x11; 16]));
        assert!(directory.discovered().is_empty());
    }

    /// Proves the cap actually evicts rather than merely stopping growth: the
    /// very first address heard must be gone once a 33rd arrives.
    #[test]
    fn the_discovered_set_evicts_the_oldest_once_it_is_full() {
        let directory = ServiceDirectory::default();
        for i in 0..=MAX_DISCOVERED {
            directory.consume_endpoint(&[i as u8; 16], None);
        }

        let discovered = directory.discovered();
        assert_eq!(discovered.len(), MAX_DISCOVERED);
        assert!(!discovered.contains(&[0u8; 16]));
        assert!(discovered.contains(&[MAX_DISCOVERED as u8; 16]));
    }

    #[test]
    fn a_withdrawal_stops_the_announce_and_keeps_the_discovered_set() {
        let directory = ServiceDirectory::default();
        directory.consume_endpoint(&[0xab; 16], None);
        directory.announce([0x22; 16]);
        assert_eq!(directory.produce_endpoint(), Some(vec![0x22; 16]));

        directory.withdraw();
        assert_eq!(directory.produce_endpoint(), None);
        assert_eq!(directory.discovered(), vec![[0xab; 16]]);
    }
}
