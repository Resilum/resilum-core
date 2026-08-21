//! TCP discovery plugin (Tor / I2P / Yggdrasil).

mod attachments;
mod endpoint;
mod plugin;
mod quota;

use attachments::Attached;
pub use attachments::Attachments;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use leviculum_std::driver::ReticulumNode;
use tokio::sync::Notify;

use crate::announce_cap::CapController;
use crate::config::{DiscoveryService, EndpointFormat};
use crate::discovery::OriginRegistry;

pub struct TcpDiscovered {
    cfg: DiscoveryService,
    engine: Arc<ReticulumNode>,
    attachments: Arc<Attachments>,
    // Fires on every successful attach so the produce loop re-announces at once.
    trigger: Arc<Notify>,
    // Persistent peer cache; None disables persistence (attach still works).
    cache_path: Option<PathBuf>,
    // Registers each attached interface for adaptive announce-cap control.
    cap_controller: Arc<CapController>,
    // Records the discovery origin (this service) of each attached interface.
    origin_registry: Arc<OriginRegistry>,
    // While false the service neither announces nor dials (its transport is down).
    active: AtomicBool,
}

impl TcpDiscovered {
    pub fn new(
        cfg: DiscoveryService,
        engine: Arc<ReticulumNode>,
        attachments: Arc<Attachments>,
        trigger: Arc<Notify>,
        cache_path: Option<PathBuf>,
        cap_controller: Arc<CapController>,
        origin_registry: Arc<OriginRegistry>,
    ) -> Self {
        // A bracketed-IPv6 (ygg) service starts dormant only in a build with a
        // runtime conduit to un-gate it (`ygg_attach`); without that — e.g. a
        // node with a real ygg tun that never attaches one — it must stay active.
        let active = if cfg!(all(unix, feature = "ygg")) {
            !matches!(cfg.endpoint_format, EndpointFormat::BracketedIpv6)
        } else {
            true
        };
        Self {
            cfg,
            engine,
            attachments,
            trigger,
            cache_path,
            cap_controller,
            origin_registry,
            active: AtomicBool::new(active),
        }
    }

    /// Bring the service up when its transport attaches: announce, and dial any
    /// cached peers now (`warm_start`).
    pub fn activate(&self) {
        self.active.store(true, Ordering::Relaxed);
        crate::discovery::warm_start(self, self.cache_path.as_deref());
    }

    /// Take the service down: drop its dialed interfaces (each detaches on drop),
    /// stop announcing, and clear the advertised endpoint so none lingers.
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Relaxed);
        self.attachments.release_service(&self.cfg.service);
        if let Some(path) = &self.cfg.hostname_path {
            let _ = std::fs::remove_file(path);
        }
    }

    fn detect_host(&self) -> Option<String> {
        if let Some(path) = self.cfg.hostname_path.as_ref() {
            let raw = std::fs::read_to_string(path).ok()?;
            let host = raw.trim();
            return (!host.is_empty()).then(|| host.to_owned());
        }
        if matches!(self.cfg.endpoint_format, EndpointFormat::BracketedIpv6) {
            return crate::net::yggdrasil_local_ipv6().map(|ip| ip.to_string());
        }
        None
    }
}
