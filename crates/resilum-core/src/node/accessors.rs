//! Read-only views onto a node's running state.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use data_encoding::HEXLOWER;
use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;

use super::Node;
use crate::config::Config;
use crate::dispatch;
use crate::egress::CandidateRegistry;
use crate::error::{Error, Result};
use crate::event::Event;
use crate::mirrors;

impl Node {
    pub fn is_running(&self) -> bool {
        self.engine.is_some()
    }

    /// The bound local SOCKS/connect port, or `0` if the connect listener is not
    /// up (no connect config, or not yet bound).
    pub fn socks_port(&self) -> u16 {
        self.socks_port.load(Ordering::Relaxed)
    }

    /// Shared engine handle for runtime tasks; `None` before start / after stop.
    pub fn engine(&self) -> Option<Arc<ReticulumNode>> {
        self.engine.clone()
    }

    /// The identity the node is running as, or `None` before start.
    pub fn identity(&self) -> Option<&Identity> {
        self.identity.as_ref()
    }

    /// The running identity's private blob (base64), or `None` before start.
    pub fn identity_base64(&self) -> Option<String> {
        self.identity.as_ref().and_then(crate::identity::to_base64)
    }

    /// `None` before start. Derived purely from the identity, so it is known
    /// without the messaging backend.
    pub fn lxmf_address_hex(&self) -> Option<String> {
        self.identity
            .as_ref()
            .map(crate::identity::lxmf_address_hex)
    }

    /// The LXMF messaging handle, or `None` when messaging is not configured or
    /// the node is not running.
    pub fn lxmf(&self) -> Option<&Arc<crate::lxmf::LxmfHandle>> {
        self.lxmf.as_ref()
    }

    /// The discovery overlay an interface was attached over (`tor` / `i2p` /
    /// `yggdrasil` / `covert`), or `None` when resilum-core did not attach it
    /// (bootstrap, LAN, or a leviculum-managed peer). Keyed by the interface id
    /// from an interface-status snapshot.
    pub fn discovered_via(&self, id: leviculum_std::InterfaceId) -> Option<String> {
        self.origin_registry.get(id)
    }

    /// Wake the discovery produce loop to re-announce endpoints now, without
    /// waiting for the next tick. Call this on external state changes this
    /// crate cannot observe from inside — an embedder posting a network-change
    /// event, a hidden-service hostname just becoming ready.
    ///
    /// The LXMF delivery destination goes out with them: a peer that has not
    /// seen its announce cannot message this node, and a network change is
    /// exactly when a new peer becomes reachable.
    pub fn trigger_discovery_announce(&self) {
        self.discovery_trigger.notify_waiters();
        if let Some(lxmf) = &self.lxmf {
            let _ = lxmf.announce();
        }
    }

    pub fn send(&mut self, _dest: &[u8], _data: &[u8]) -> Result<()> {
        if self.engine.is_none() {
            return Err(Error::NotRunning);
        }
        Ok(())
    }

    /// The poisoned lock is recovered from: it is only ever held across a push
    /// or a pop, and `None` would claim the node had nothing to report.
    pub fn poll_event(&mut self) -> Option<Event> {
        self.event_queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The shared egress candidate registry.
    pub fn registry(&self) -> &Arc<CandidateRegistry> {
        &self.registry
    }

    /// The node-event bus; subsystems subscribe to receive engine events.
    pub fn events(&self) -> &dispatch::Events {
        &self.events
    }

    /// Mesh-discovered rngit mirror advertisements from peers; `None` before start.
    pub fn mirror_registry(&self) -> Option<&Arc<mirrors::Registry>> {
        self.mirror_registry.as_ref()
    }

    /// Nostr bridges heard on the mesh (hex LXMF addresses), so a caller can
    /// offer one as an upstream without the user typing it in by hand. Empty
    /// until at least one bridge announces itself.
    #[must_use]
    pub fn nostr_relays(&self) -> Vec<String> {
        self.nostr_relay
            .discovered()
            .iter()
            .map(|addr| HEXLOWER.encode(addr))
            .collect()
    }

    /// Start or stop announcing this node as a Nostr bridge. A bridge crate
    /// calls this once the node is running and it knows both its own LXMF
    /// address and whether its configuration says to publish; `address:
    /// None` withdraws the announce on the next tick without forgetting what
    /// this node has discovered of other bridges.
    pub fn advertise_nostr_relay(&self, address: Option<[u8; 16]>) {
        self.nostr_relay.set_advertise(address);
    }
}
