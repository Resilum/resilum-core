use std::collections::BTreeSet;

use leviculum_std::InterfaceId;

pub(super) struct Targets {
    configured: Vec<String>,
    discovered: BTreeSet<String>,
    interface: Option<InterfaceId>,
}

impl Targets {
    pub(super) fn configured(peers: &[String]) -> Self {
        Self {
            configured: peers.to_vec(),
            discovered: BTreeSet::new(),
            interface: None,
        }
    }

    pub(super) fn add(&mut self, peer: String) -> bool {
        !self.configured.contains(&peer) && self.discovered.insert(peer)
    }

    pub(super) fn forget(&mut self, peer: &str) {
        self.discovered.remove(peer);
    }

    pub(super) fn all(&self) -> Vec<String> {
        self.configured
            .iter()
            .cloned()
            .chain(self.discovered.iter().cloned())
            .collect()
    }

    pub(super) fn now_served_by(&mut self, id: Option<InterfaceId>) -> Option<InterfaceId> {
        std::mem::replace(&mut self.interface, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peers(of: &[&str]) -> Vec<String> {
        of.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn a_discovered_peer_joins_the_configured_ones() {
        let mut targets = Targets::configured(&peers(&["a:1"]));

        assert!(targets.add("b:2".into()));

        assert_eq!(targets.all(), peers(&["a:1", "b:2"]));
    }

    #[test]
    fn a_peer_announcing_twice_does_not_rebuild_the_interface() {
        let mut targets = Targets::configured(&[]);
        assert!(targets.add("b:2".into()));

        assert!(!targets.add("b:2".into()));
    }

    #[test]
    fn a_peer_already_configured_by_hand_is_not_added_again() {
        let mut targets = Targets::configured(&peers(&["a:1"]));

        assert!(!targets.add("a:1".into()));

        assert_eq!(targets.all(), peers(&["a:1"]));
    }

    #[test]
    fn a_peer_that_left_stops_receiving_but_a_configured_one_stays() {
        let mut targets = Targets::configured(&peers(&["a:1"]));
        targets.add("b:2".into());

        targets.forget("b:2");
        targets.forget("a:1");

        assert_eq!(targets.all(), peers(&["a:1"]));
    }
}
