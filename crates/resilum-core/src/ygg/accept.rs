//! Inbound RNS links arriving over the overlay.

use std::net::{Ipv6Addr, SocketAddr};
use std::sync::Arc;

use leviculum_std::driver::ReticulumNode;
use tokio_smoltcp::Net;

use super::Links;
use crate::discovery::OriginRegistry;

pub(super) async fn accept(
    engine: Arc<ReticulumNode>,
    origin: Arc<OriginRegistry>,
    net: Arc<Net>,
    address: Ipv6Addr,
    rns_port: u16,
    links: Links,
) {
    let mut listener = match net
        .tcp_bind(SocketAddr::new(address.into(), rns_port))
        .await
    {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(error = %e, "ygg tcp_bind failed");
            return;
        }
    };
    while let Ok((stream, remote)) = listener.accept().await {
        let ip = remote.ip();
        let name = format!("ygg[{ip}]");
        match engine.spawn_byte_channel(&name, stream) {
            Ok(handle) => {
                origin.record(handle.id(), "yggdrasil");
                tracing::info!(%name, "attached RNS peer over yggdrasil");
                // One link per peer: inserting drops any prior handle for this
                // address, detaching a stale link the peer is re-dialing over.
                links.lock().expect("ygg links").insert(ip, handle);
            }
            Err(e) => tracing::warn!(%name, error = %e, "ygg byte-channel attach failed"),
        }
    }
}
