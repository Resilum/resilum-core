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
mod resolvers;
#[cfg(feature = "iroh-protect")]
mod transport;
mod wiring;

pub(crate) use plugin::IrohDiscovery;

use std::path::Path;
use std::sync::Arc;

use iroh::Endpoint;
use leviculum_std::driver::ReticulumNode;
use tokio::task::JoinHandle;

use crate::config::IrohConfig;
use wiring::Wiring;

pub(super) const ORIGIN: &str = "iroh";

/// A live iroh attachment; drop or [`IrohHandle::detach`] to tear it down.
#[must_use]
pub struct IrohHandle {
    endpoint: Endpoint,
    tasks: Vec<JoinHandle<()>>,
    wiring: Arc<Wiring>,
    discovery: Option<Arc<IrohDiscovery>>,
    runtime: tokio::runtime::Handle,
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
        self.wiring.attachments.release_service(ORIGIN);
        // Close gracefully so iroh sends CONNECTION_CLOSE rather than logging an
        // ungraceful abort on drop. Best-effort: skip when already on a runtime
        // thread, where `block_on` would panic.
        if tokio::runtime::Handle::try_current().is_err() {
            self.runtime.block_on(self.endpoint.close());
        }
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
    engine: Arc<ReticulumNode>,
    dir: &Path,
    cfg: &IrohConfig,
    discovery: Option<Arc<IrohDiscovery>>,
    protect: Option<leviculum_std::socket_hook::OutboundSocketHook>,
    attachments: Arc<crate::discovery::Attachments>,
    origin: Arc<crate::discovery::OriginRegistry>,
) -> Result<IrohHandle, String> {
    let secret = key::load_or_create(dir);
    let endpoint = engine::build(secret, cfg, protect).await?;
    let wiring = Arc::new(Wiring {
        engine,
        attachments,
        origin,
    });
    let mut tasks = vec![tokio::spawn(accept::run(endpoint.clone(), wiring.clone()))];
    for peer in &cfg.bootstrap {
        tasks.push(tokio::spawn(dial::bootstrap(
            endpoint.clone(),
            wiring.clone(),
            peer.clone(),
        )));
    }
    if let Some(discovery) = &discovery {
        discovery.activate(endpoint.clone(), wiring.clone());
    }
    Ok(IrohHandle {
        endpoint,
        tasks,
        wiring,
        discovery,
        runtime: tokio::runtime::Handle::current(),
    })
}
