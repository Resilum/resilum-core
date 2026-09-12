//! Announce-driven peer discovery: one announce carries every transport this
//! node has ready, and each endpoint in it routes to the plugin for its
//! service.

pub(crate) mod admit;
pub(crate) mod attachments;
mod build;
mod consume;
pub mod covert;
mod directory;
mod endpoint;
mod origin;
mod produce;
mod quota;
pub mod service;
pub(crate) mod store;
mod tcp;
mod udp;
pub use attachments::{Attachments, Link};
pub use build::{BuildParams, build_covert_addresses, build_from_services};
pub use consume::run_consume;
pub use directory::ServiceDirectory;
pub use origin::OriginRegistry;
pub use produce::{build_destination, run_produce};
pub use store::run_prune_loop;
pub use tcp::TcpDiscovered;
pub use udp::UdpDiscovered;

use std::collections::BTreeMap;
use std::sync::Arc;

use leviculum_std::api::Destination;

use crate::config::DiscoveryService;
pub use service::Service;

pub(super) const APP_NAME: &str = "resilum";

/// A transport-specific discovery plugin.
pub trait DiscoveryPlugin: Send + Sync {
    /// Bytes advertising where we accept peers over this transport, or `None`
    /// while the local transport is not ready.
    fn produce_endpoint(&self) -> Option<Vec<u8>>;
    /// React to a peer advertising the same transport.
    ///
    /// `announcer_pubkey` is the identity that signed the announce, and `None`
    /// when the endpoint came from the on-disk cache instead of a live
    /// announce — there is no signer to name then. Transports that need the
    /// key to seal a session can do nothing with such an endpoint; the ones
    /// that dial an address (TCP discovery) do not look at it.
    fn consume_endpoint(&self, payload: &[u8], announcer_pubkey: Option<&[u8]>);
    fn forget_stale_peers(&self, _now: f64) {}
}

/// Plugins keyed by the service they speak for, which is how a peer names its
/// endpoints in the announce.
#[derive(Default)]
pub struct Discovery {
    by_service: BTreeMap<Service, Arc<dyn DiscoveryPlugin>>,
}

impl Discovery {
    pub fn register(&mut self, service: Service, plugin: Arc<dyn DiscoveryPlugin>) {
        self.by_service.insert(service, plugin);
    }

    /// Hand each endpoint in an announce to the plugin for its service; a
    /// service this node does not run is not an error.
    pub fn on_announce(&self, endpoints: &BTreeMap<String, Vec<u8>>, announcer_pubkey: &[u8]) {
        for (service, endpoint) in endpoints {
            let Some(plugin) = Service::from_name(service).and_then(|s| self.by_service.get(&s))
            else {
                continue;
            };
            plugin.consume_endpoint(endpoint, Some(announcer_pubkey));
        }
    }

    pub fn forget_stale_peers(&self, now: f64) {
        for plugin in self.by_service.values() {
            plugin.forget_stale_peers(now);
        }
    }

    /// Every service whose transport is ready, for the announce loop.
    pub fn endpoints(&self) -> BTreeMap<String, Vec<u8>> {
        self.by_service
            .iter()
            .filter_map(|(service, plugin)| {
                Some((service.name().to_owned(), plugin.produce_endpoint()?))
            })
            .collect()
    }
}

/// Name-hash of the aspect `resilum.discovery.<service>`.
pub fn name_hash(service: &str) -> Vec<u8> {
    Destination::compute_name_hash(APP_NAME, &["discovery", service]).to_vec()
}

pub(crate) fn warm_start(plugin: &dyn DiscoveryPlugin, remembered: &store::Peers) {
    remembered.forget_stale(crate::wall_clock::unix_now());
    for endpoint in remembered.most_recent() {
        plugin.consume_endpoint(&endpoint, None);
    }
}

/// The list of enabled service names, for the prune loop.
pub fn service_names(services: &[DiscoveryService]) -> Vec<String> {
    services.iter().map(|s| s.service.clone()).collect()
}

#[cfg(test)]
mod tests;
