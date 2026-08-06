//! In-process iroh QUIC transport: dial peers by `EndpointId` with NAT
//! hole-punching and relay fallback, bridging each connection onto a leviculum
//! byte-channel so an RNS link rides it — the connectivity sibling of the
//! Yggdrasil underlay, not an anonymity transport.

mod accept;
mod bridge;
mod dial;
mod engine;
mod key;
mod plugin;
#[cfg(feature = "iroh-protect")]
mod transport;

pub(crate) use plugin::IrohDiscovery;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use iroh::{Endpoint, EndpointId};
use leviculum_std::api::Node as LevNode;
use leviculum_std::interfaces::ByteChannelHandle;
use tokio::task::JoinHandle;

use crate::config::IrohConfig;

/// One byte-channel per peer, so a re-dial replaces a stale link and teardown
/// can detach them all.
pub(super) type Links = Arc<Mutex<HashMap<EndpointId, ByteChannelHandle>>>;

/// A live iroh attachment; drop or [`IrohHandle::detach`] to tear it down.
#[must_use]
pub struct IrohHandle {
    endpoint: Endpoint,
    tasks: Vec<JoinHandle<()>>,
    links: Links,
    engine: Arc<LevNode>,
    discovery: Option<Arc<IrohDiscovery>>,
}

impl IrohHandle {
    /// This node's `EndpointId` — the address peers dial us on.
    pub fn endpoint_id(&self) -> String {
        self.endpoint.id().to_string()
    }

    /// Tear the transport down now instead of on drop.
    pub fn detach(mut self) {
        self.teardown();
    }

    fn teardown(&mut self) {
        if let Some(discovery) = self.discovery.take() {
            discovery.deactivate();
        }
        for task in std::mem::take(&mut self.tasks) {
            task.abort();
        }
        let mut links = self.links.lock().expect("iroh links");
        for handle in links.values() {
            let _ = self.engine.remove_interface(handle.id());
        }
        links.clear();
    }
}

impl Drop for IrohHandle {
    fn drop(&mut self) {
        self.teardown();
    }
}

/// Build the endpoint from `cfg`, accept inbound iroh links, dial the bootstrap
/// peers, and wire the warm-discovery plugin (if any) to the live transport.
/// `dir` holds the persisted secret. Must run inside the node runtime.
pub async fn attach(
    engine: Arc<LevNode>,
    dir: &Path,
    cfg: &IrohConfig,
    discovery: Option<Arc<IrohDiscovery>>,
    protect: Option<leviculum_std::socket_hook::OutboundSocketHook>,
) -> Result<IrohHandle, String> {
    let secret = key::load_or_create(dir);
    let endpoint = engine::build(secret, cfg, protect).await?;
    let links: Links = Arc::new(Mutex::new(HashMap::new()));
    let mut tasks = vec![tokio::spawn(accept::run(
        endpoint.clone(),
        engine.clone(),
        links.clone(),
    ))];
    for peer in &cfg.bootstrap {
        tasks.push(tokio::spawn(dial::bootstrap(
            endpoint.clone(),
            engine.clone(),
            links.clone(),
            peer.clone(),
        )));
    }
    if let Some(discovery) = &discovery {
        discovery.activate(endpoint.clone(), engine.clone(), links.clone());
    }
    Ok(IrohHandle {
        endpoint,
        tasks,
        links,
        engine,
        discovery,
    })
}
