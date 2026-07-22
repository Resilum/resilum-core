//! Announce-driven peer discovery: each plugin advertises one transport
//! endpoint and reacts to peers advertising the same `resilum.discovery.<svc>`
//! aspect. Incoming announces route to a plugin by their name-hash.

mod consume;
mod produce;
mod tcp;
pub use consume::run_consume;
pub use produce::{build_destinations, run_produce};
pub use tcp::TcpDiscovered;

use std::collections::HashMap;
use std::sync::Arc;

use leviculum_std::api::Destination;
use leviculum_std::api::Node as LevNode;
use tokio::sync::Notify;

use crate::config::DiscoveryService;

pub(super) const APP_NAME: &str = "resilum";

/// A transport-specific discovery plugin.
pub trait DiscoveryPlugin: Send + Sync {
    /// Bytes advertising where we accept peers over this transport, or `None`
    /// while the local transport is not ready.
    fn produce_endpoint(&self) -> Option<Vec<u8>>;
    /// React to a peer advertising the same transport (e.g. add an interface).
    fn consume_endpoint(&self, payload: &[u8]);
}

/// Plugins keyed by the name-hash of their discovery aspect.
#[derive(Default)]
pub struct Discovery {
    by_name_hash: HashMap<Vec<u8>, Arc<dyn DiscoveryPlugin>>,
    services: HashMap<Vec<u8>, String>,
}

impl Discovery {
    pub fn register(&mut self, service: &str, plugin: Arc<dyn DiscoveryPlugin>) {
        let nh = name_hash(service);
        self.services.insert(nh.clone(), service.to_owned());
        self.by_name_hash.insert(nh, plugin);
    }

    /// Route an announce to the matching plugin's `consume_endpoint`.
    pub fn on_announce(&self, name_hash: &[u8], app_data: &[u8]) {
        if let Some(plugin) = self.by_name_hash.get(name_hash) {
            plugin.consume_endpoint(app_data);
        }
    }

    /// Each registered service with its current produced endpoint (for the
    /// announce loop); services whose transport is not ready are skipped.
    pub fn endpoints(&self) -> Vec<(String, Vec<u8>)> {
        self.by_name_hash
            .iter()
            .filter_map(|(nh, plugin)| {
                let service = self.services.get(nh)?.clone();
                Some((service, plugin.produce_endpoint()?))
            })
            .collect()
    }
}

/// Name-hash of the aspect `resilum.discovery.<service>`.
pub fn name_hash(service: &str) -> Vec<u8> {
    Destination::compute_name_hash(APP_NAME, &["discovery", service]).to_vec()
}

pub fn build_from_services(
    services: &[DiscoveryService],
    engine: Arc<LevNode>,
    trigger: Arc<Notify>,
) -> Discovery {
    let mut d = Discovery::default();
    for cfg in services {
        d.register(
            &cfg.service.clone(),
            Arc::new(TcpDiscovered::new(
                cfg.clone(),
                engine.clone(),
                trigger.clone(),
            )),
        );
    }
    d
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
        fn consume_endpoint(&self, payload: &[u8]) {
            self.consumed.lock().unwrap().push(payload.to_vec());
        }
    }

    #[test]
    fn routes_matching_announce_only() {
        let plugin = Arc::new(Fake::default());
        let mut d = Discovery::default();
        d.register("tor", plugin.clone());

        d.on_announce(&name_hash("tor"), b"payload");
        d.on_announce(&name_hash("i2p"), b"other"); // no plugin → ignored

        assert_eq!(
            plugin.consumed.lock().unwrap().as_slice(),
            &[b"payload".to_vec()]
        );
    }

    #[test]
    fn endpoints_lists_registered_services() {
        let mut d = Discovery::default();
        d.register("tor", Arc::new(Fake::default()));
        let eps = d.endpoints();
        assert_eq!(eps.len(), 1);
        assert_eq!(eps[0].0, "tor");
        assert_eq!(eps[0].1, b"endpoint");
    }
}
