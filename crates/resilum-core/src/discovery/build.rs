//! `Discovery` construction from config: plugin registration for each
//! configured TCP or covert carrier.

use std::sync::Arc;

use leviculum_std::driver::ReticulumNode;
use tokio::sync::Notify;

use super::{Discovery, OriginRegistry, Service, TcpDiscovered, covert, store};
use crate::announce_cap::CapController;
use crate::config::{CovertDiscoveryService, DiscoveryService, EndpointFormat};

pub struct BuildParams<'a> {
    pub tcp: &'a [DiscoveryService],
    pub covert: &'a [CovertDiscoveryService],
    pub covert_addresses: &'a [Arc<covert::AddressSource>],
    pub udp: Option<&'a crate::config::UdpInterface>,
    pub engine: Arc<ReticulumNode>,
    pub coordinates: Arc<crate::coordinates::Coordinates>,
    pub attachments: Arc<super::Attachments>,
    pub trigger: Arc<Notify>,
    pub storage_root: Option<&'a std::path::Path>,
    pub cap_controller: Arc<CapController>,
    pub events: crate::dispatch::Events,
    pub origin_registry: Arc<OriginRegistry>,
    pub nursery: Arc<crate::nursery::Nursery>,
}

/// Builds the discovery plugin set, plus the yggdrasil plugin (if configured),
/// which the node activates/deactivates as its conduit attaches.
pub fn build_from_services(p: BuildParams<'_>) -> (Discovery, Option<Arc<TcpDiscovered>>) {
    let mut d = Discovery::default();
    let mut ygg = None;
    let attachments = p.attachments.clone();
    for cfg in p.tcp {
        let Some(service) = named(&cfg.service) else {
            continue;
        };
        let cache_path = p.storage_root.map(|r| store::path_for(r, &cfg.service));
        let plugin = Arc::new(TcpDiscovered::new(
            cfg.clone(),
            p.engine.clone(),
            attachments.clone(),
            p.trigger.clone(),
            cache_path,
            p.cap_controller.clone(),
            p.origin_registry.clone(),
        ));
        plugin.dial_whoever_we_remember();
        if matches!(cfg.endpoint_format, EndpointFormat::BracketedIpv6) {
            ygg = Some(plugin.clone());
        }
        d.register(service, plugin);
    }
    if let Some(udp) = p.udp {
        let (_, port) = udp.bound_to().rsplit_once(':').unwrap_or_default();
        d.register(
            Service::UDP,
            Arc::new(super::UdpDiscovered::new(
                DiscoveryService::udp(port.parse().unwrap_or(4242)),
                udp,
                p.engine.clone(),
                attachments.clone(),
                p.origin_registry.clone(),
                Arc::new(covert::AddressSource::new(udp.reachable_on.clone())),
            )),
        );
    }
    for (cfg, addresses) in p.covert.iter().zip(p.covert_addresses) {
        let Some(service) = named(&cfg.service_name()) else {
            continue;
        };
        let plugin = Arc::new(covert::CovertDiscovered::new(
            cfg.clone(),
            addresses.clone(),
            p.engine.clone(),
            p.events.clone(),
            p.origin_registry.clone(),
            attachments.clone(),
            p.nursery.clone(),
        ));
        d.register(service, plugin);
    }
    (d, ygg)
}

/// The one place a configured name becomes a service. A name outside the wire
/// vocabulary could never be announced nor matched against a peer's announce,
/// so the plugin is refused here instead of running dead.
fn named(service: &str) -> Option<Service> {
    let known = Service::from_name(service);
    if known.is_none() {
        tracing::error!(%service, "no such discovery service; plugin not registered");
    }
    known
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
