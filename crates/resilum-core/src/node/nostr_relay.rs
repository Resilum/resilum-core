use data_encoding::HEXLOWER;

use super::Node;

impl Node {
    /// Hex LXMF addresses. Empty until at least one bridge announces itself,
    /// which is not an error.
    #[must_use]
    pub fn nostr_relays(&self) -> Vec<String> {
        self.nostr_relay
            .discovered()
            .iter()
            .map(|addr| HEXLOWER.encode(addr))
            .collect()
    }

    /// `None` withdraws the announce on the next tick without forgetting what
    /// this node has discovered of other bridges.
    pub fn advertise_nostr_relay(&self, address: Option<[u8; 16]>) {
        self.nostr_relay.set_advertise(address);
    }
}
