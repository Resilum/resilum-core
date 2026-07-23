//! Covert-carrier discovery plugin. Skeleton: rendezvous fetch + pipe attach
//! land in the following slices.

use std::sync::Arc;

use leviculum_std::api::Node as LevNode;

use super::super::DiscoveryPlugin;
use crate::config::CovertDiscoveryService;

pub struct CovertDiscovered {
    cfg: CovertDiscoveryService,
    #[allow(dead_code)]
    engine: Arc<LevNode>,
}

impl CovertDiscovered {
    pub fn new(cfg: CovertDiscoveryService, engine: Arc<LevNode>) -> Self {
        Self { cfg, engine }
    }
}

impl DiscoveryPlugin for CovertDiscovered {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        // Empty payload = capability marker; the real address is fetched over
        // the encrypted link by the rendezvous client.
        (!self.cfg.addresses.is_empty()).then(Vec::new)
    }

    fn consume_endpoint(&self, _payload: &[u8], announcer_pubkey: &[u8]) {
        tracing::debug!(
            carrier = %self.cfg.carrier,
            pubkey_head = %format_head(announcer_pubkey),
            "covert peer announced; rendezvous fetch not yet wired"
        );
    }
}

fn format_head(bytes: &[u8]) -> String {
    bytes.iter().take(4).fold(String::new(), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}
