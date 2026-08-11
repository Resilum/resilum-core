//! Read-only views onto a node's running state.

use std::sync::Arc;
use std::sync::atomic::Ordering;

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

    /// This node's LXMF delivery address (hex), or `None` before start. Derived
    /// purely from the identity, so it is known without the messaging backend.
    pub fn lxmf_address(&self) -> Option<String> {
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
    /// waiting for the next tick. Call this on external state changes the
    /// bridge cannot observe from inside (Flutter posting a network-change
    /// event through FFI, a hidden-service hostname just becoming ready, etc).
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

    pub fn poll_event(&mut self) -> Option<Event> {
        self.event_queue.lock().expect("event queue").pop_front()
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
}
