//! TCP discovery plugin (Tor / I2P / Yggdrasil).

mod plugin;

use super::attachments::{Attached, Attachments};

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use leviculum_std::driver::ReticulumNode;
use tokio::sync::Notify;

use crate::announce_cap::CapController;
use crate::config::{DiscoveryService, EndpointFormat};
use crate::discovery::OriginRegistry;
use crate::discovery::store::{Advertised, Peers};

pub struct TcpDiscovered {
    cfg: DiscoveryService,
    engine: Arc<ReticulumNode>,
    attachments: Arc<Attachments>,
    announce_again: Arc<Notify>,
    remembered: Peers,
    advertised: Advertised,
    cap_controller: Arc<CapController>,
    origin_registry: Arc<OriginRegistry>,
    transport_is_up: AtomicBool,
}

impl TcpDiscovered {
    pub fn new(
        cfg: DiscoveryService,
        engine: Arc<ReticulumNode>,
        attachments: Arc<Attachments>,
        announce_again: Arc<Notify>,
        cache_path: Option<PathBuf>,
        cap_controller: Arc<CapController>,
        origin_registry: Arc<OriginRegistry>,
    ) -> Self {
        let transport_is_up = AtomicBool::new(!waits_for_a_conduit_to_be_attached(&cfg));
        let advertised = Advertised::at(cfg.hostname_path.clone());
        Self {
            cfg,
            engine,
            attachments,
            announce_again,
            remembered: Peers::open(cache_path),
            advertised,
            cap_controller,
            origin_registry,
            transport_is_up,
        }
    }

    pub fn activate(&self) {
        self.transport_is_up.store(true, Ordering::Relaxed);
        self.dial_whoever_we_remember();
    }

    pub(crate) fn dial_whoever_we_remember(&self) {
        crate::discovery::warm_start(self, &self.remembered);
    }

    pub fn deactivate(&self) {
        self.transport_is_up.store(false, Ordering::Relaxed);
        self.attachments.release_service(&self.cfg.service);
        self.advertised.forget();
    }

    fn detect_host(&self) -> Option<String> {
        if self.advertised.is_served_from_a_file() {
            return self.advertised.read();
        }
        if matches!(self.cfg.endpoint_format, EndpointFormat::BracketedIpv6) {
            return crate::net::yggdrasil_local_ipv6().map(|ip| ip.to_string());
        }
        None
    }
}

fn waits_for_a_conduit_to_be_attached(cfg: &DiscoveryService) -> bool {
    cfg!(all(unix, feature = "ygg")) && matches!(cfg.endpoint_format, EndpointFormat::BracketedIpv6)
}
