//! `Discovery` construction from config: plugin registration for each
//! configured TCP or covert carrier.

use std::sync::Arc;

use leviculum_std::driver::ReticulumNode;
use tokio::sync::Notify;

use super::{Discovery, OriginRegistry, TcpDiscovered, cache, covert};
use crate::announce_cap::CapController;
use crate::config::{CovertDiscoveryService, DiscoveryService, EndpointFormat};

pub struct BuildParams<'a> {
    pub tcp: &'a [DiscoveryService],
    pub covert: &'a [CovertDiscoveryService],
    pub covert_addresses: &'a [Arc<covert::AddressSource>],
    pub engine: Arc<ReticulumNode>,
    pub trigger: Arc<Notify>,
    pub storage_root: Option<&'a std::path::Path>,
    pub cap_controller: Arc<CapController>,
    pub events: crate::dispatch::Events,
    pub origin_registry: Arc<OriginRegistry>,
}

/// Builds the discovery plugin set, plus the yggdrasil plugin (if configured),
/// which the node activates/deactivates as its conduit attaches.
pub fn build_from_services(p: BuildParams<'_>) -> (Discovery, Option<Arc<TcpDiscovered>>) {
    let mut d = Discovery::default();
    let mut ygg = None;
    for cfg in p.tcp {
        let cache_path = p.storage_root.map(|r| cache::path_for(r, &cfg.service));
        let plugin = Arc::new(TcpDiscovered::new(
            cfg.clone(),
            p.engine.clone(),
            p.trigger.clone(),
            cache_path.clone(),
            p.cap_controller.clone(),
            p.origin_registry.clone(),
        ));
        super::warm_start(plugin.as_ref(), cache_path.as_deref());
        if matches!(cfg.endpoint_format, EndpointFormat::BracketedIpv6) {
            ygg = Some(plugin.clone());
        }
        d.register(&cfg.service.clone(), plugin);
    }
    for (cfg, addresses) in p.covert.iter().zip(p.covert_addresses) {
        let name = cfg.service_name();
        let plugin = Arc::new(covert::CovertDiscovered::new(
            cfg.clone(),
            addresses.clone(),
            p.engine.clone(),
            p.events.clone(),
            p.origin_registry.clone(),
        ));
        d.register(&name, plugin);
    }
    (d, ygg)
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
