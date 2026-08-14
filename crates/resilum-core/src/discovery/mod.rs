//! Announce-driven peer discovery: one announce carries every transport this
//! node has ready, and each endpoint in it routes to the plugin for its
//! service.

mod build;
mod cache;
mod consume;
pub mod covert;
mod origin;
mod produce;
pub mod service;
mod tcp;
pub use build::{BuildParams, build_covert_addresses, build_from_services};
pub use cache::run_prune_loop;
pub use consume::run_consume;
pub use origin::OriginRegistry;
pub use produce::{build_destination, run_produce};
pub use tcp::TcpDiscovered;

use std::collections::BTreeMap;
use std::sync::Arc;

use leviculum_std::api::Destination;

use crate::config::DiscoveryService;

pub(super) const APP_NAME: &str = "resilum";

/// A transport-specific discovery plugin.
pub trait DiscoveryPlugin: Send + Sync {
    /// Bytes advertising where we accept peers over this transport, or `None`
    /// while the local transport is not ready.
    fn produce_endpoint(&self) -> Option<Vec<u8>>;
    /// React to a peer advertising the same transport. `announcer_pubkey` is
    /// the identity that signed the announce (some transports need it to seal
    /// a session; TCP-discovery doesn't).
    fn consume_endpoint(&self, payload: &[u8], announcer_pubkey: &[u8]);
}

/// Plugins keyed by the service they speak for, which is how a peer names its
/// endpoints in the announce.
#[derive(Default)]
pub struct Discovery {
    by_service: BTreeMap<String, Arc<dyn DiscoveryPlugin>>,
}

impl Discovery {
    pub fn register(&mut self, service: &str, plugin: Arc<dyn DiscoveryPlugin>) {
        self.by_service.insert(service.to_owned(), plugin);
    }

    /// Hand each endpoint in an announce to the plugin for its service; a
    /// service this node does not run is not an error.
    pub fn on_announce(&self, endpoints: &BTreeMap<String, Vec<u8>>, announcer_pubkey: &[u8]) {
        for (service, endpoint) in endpoints {
            if let Some(plugin) = self.by_service.get(service) {
                plugin.consume_endpoint(endpoint, announcer_pubkey);
            }
        }
    }

    /// Every service whose transport is ready, for the announce loop.
    pub fn endpoints(&self) -> BTreeMap<String, Vec<u8>> {
        self.by_service
            .iter()
            .filter_map(|(service, plugin)| Some((service.clone(), plugin.produce_endpoint()?)))
            .collect()
    }
}

/// Name-hash of the aspect `resilum.discovery.<service>`.
pub fn name_hash(service: &str) -> Vec<u8> {
    Destination::compute_name_hash(APP_NAME, &["discovery", service]).to_vec()
}

pub(crate) fn warm_start(plugin: &dyn DiscoveryPlugin, cache_path: Option<&std::path::Path>) {
    let Some(path) = cache_path else { return };
    let mut records = cache::load(path);
    cache::prune(&mut records, cache::TTL_SECONDS, cache::now_ts());
    let _ = cache::save(path, &records);
    for endpoint in cache::top_n(&records, cache::TOP_N_ACTIVE) {
        plugin.consume_endpoint(&endpoint, &[]);
    }
}

/// The list of enabled service names, for the prune loop.
pub fn service_names(services: &[DiscoveryService]) -> Vec<String> {
    services.iter().map(|s| s.service.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake {
        consumed: Mutex<Vec<Vec<u8>>>,
    }

    impl DiscoveryPlugin for Fake {
        fn produce_endpoint(&self) -> Option<Vec<u8>> {
            Some(b"endpoint".to_vec())
        }
        fn consume_endpoint(&self, payload: &[u8], _announcer_pubkey: &[u8]) {
            self.consumed.lock().unwrap().push(payload.to_vec());
        }
    }

    fn announced(pairs: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
        pairs
            .iter()
            .map(|(s, e)| ((*s).to_owned(), e.to_vec()))
            .collect()
    }

    #[test]
    fn each_endpoint_reaches_the_plugin_for_its_service() {
        let plugin = Arc::new(Fake::default());
        let mut d = Discovery::default();
        d.register("tor", plugin.clone());

        // One announce, two services: the one nothing is registered for is
        // another node's transport, not an error.
        d.on_announce(
            &announced(&[("tor", b"payload"), ("i2p", b"other")]),
            b"pubkey",
        );

        assert_eq!(
            plugin.consumed.lock().unwrap().as_slice(),
            &[b"payload".to_vec()]
        );
    }

    #[test]
    fn endpoints_lists_registered_services() {
        let mut d = Discovery::default();
        d.register("tor", Arc::new(Fake::default()));
        assert_eq!(d.endpoints(), announced(&[("tor", b"endpoint")]));
    }
}
