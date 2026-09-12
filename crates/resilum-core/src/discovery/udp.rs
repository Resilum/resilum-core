mod plugin;
mod targets;

use std::sync::{Arc, Mutex};

use leviculum_std::InterfaceId;
use leviculum_std::api::InterfaceConfig;
use leviculum_std::driver::ReticulumNode;

use super::attachments::Attachments;
use super::covert::AddressSource;
use crate::config::{DiscoveryService, UdpInterface};
use crate::discovery::OriginRegistry;
use targets::Targets;

pub struct UdpDiscovered {
    inner: Arc<Inner>,
}

struct Inner {
    cfg: DiscoveryService,
    engine: Arc<ReticulumNode>,
    attachments: Arc<Attachments>,
    origin_registry: Arc<OriginRegistry>,
    ours: Arc<AddressSource>,
    bound_to: String,
    targets: Mutex<Targets>,
}

impl UdpDiscovered {
    pub fn new(
        cfg: DiscoveryService,
        udp: &UdpInterface,
        engine: Arc<ReticulumNode>,
        attachments: Arc<Attachments>,
        origin_registry: Arc<OriginRegistry>,
        ours: Arc<AddressSource>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                cfg,
                engine,
                attachments,
                origin_registry,
                ours,
                bound_to: udp.bound_to().to_owned(),
                targets: Mutex::new(Targets::configured(&udp.peers_every_datagram_goes_to)),
            }),
        }
    }
}

struct PeerLeavesWhenDropped {
    inner: Arc<Inner>,
    peer: String,
}

impl Drop for PeerLeavesWhenDropped {
    fn drop(&mut self) {
        self.inner
            .targets
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .forget(&self.peer);
        self.inner.rebuild();
    }
}

impl Inner {
    fn rebuild(&self) -> Option<InterfaceId> {
        let mut targets = self.targets.lock().unwrap_or_else(|e| e.into_inner());
        let spawned = self.spawn(&targets.all());
        if let Some(previous) = targets.now_served_by(spawned) {
            let _ = self.engine.remove_interface(previous);
        }
        spawned
    }

    fn spawn(&self, peers: &[String]) -> Option<InterfaceId> {
        let (host, port) = self.bound_to.rsplit_once(':')?;
        if peers.is_empty() {
            return None;
        }
        let config = InterfaceConfig {
            name: self.cfg.name_prefix.clone(),
            interface_type: "UDPInterface".to_owned(),
            listen_ip: Some(host.to_owned()),
            listen_port: port.parse().ok(),
            forward_ip: Some(peers.join(", ")),
            ..Default::default()
        };
        match self.engine.spawn_interface(config) {
            Ok(ids) => ids.first().copied(),
            Err(e) => {
                tracing::warn!(service = %self.cfg.service, error = %e, "udp interface failed");
                None
            }
        }
    }
}
