//! `Discovery` construction from config: plugin registration for each
//! configured TCP or covert carrier.

use std::sync::Arc;

use leviculum_std::api::Node as LevNode;
use tokio::sync::Notify;

use super::{Discovery, TcpDiscovered, cache, covert};
use crate::announce_cap::CapController;
use crate::config::{CovertDiscoveryService, DiscoveryService};

pub struct BuildParams<'a> {
    pub tcp: &'a [DiscoveryService],
    pub covert: &'a [CovertDiscoveryService],
    pub covert_addresses: &'a [Arc<covert::AddressSource>],
    pub engine: Arc<LevNode>,
    pub trigger: Arc<Notify>,
    pub storage_root: Option<&'a std::path::Path>,
    pub cap_controller: Arc<CapController>,
    pub events: crate::dispatch::Events,
}

pub fn build_from_services(p: BuildParams<'_>) -> Discovery {
    let mut d = Discovery::default();
    for cfg in p.tcp {
        let cache_path = p.storage_root.map(|r| cache::path_for(r, &cfg.service));
        let plugin = Arc::new(TcpDiscovered::new(
            cfg.clone(),
            p.engine.clone(),
            p.trigger.clone(),
            cache_path.clone(),
            p.cap_controller.clone(),
        ));
        super::warm_start(plugin.as_ref(), cache_path.as_deref());
        d.register(&cfg.service.clone(), plugin);
    }
    for (cfg, addresses) in p.covert.iter().zip(p.covert_addresses) {
        let name = cfg.service_name();
        let plugin = Arc::new(covert::CovertDiscovered::new(
            cfg.clone(),
            addresses.clone(),
            p.engine.clone(),
            p.events.clone(),
        ));
        d.register(&name, plugin);
    }
    d
}

/// One [`covert::AddressSource`] per configured covert carrier. Shared between
/// the plugin's `produce_endpoint` and the responder task, so both see the
/// same effective addresses.
pub fn build_covert_addresses(
    covert_services: &[CovertDiscoveryService],
) -> Vec<Arc<covert::AddressSource>> {
    covert_services
        .iter()
        .map(|c| Arc::new(covert::AddressSource::new(c.addresses.clone())))
        .collect()
}
