//! Outbound dials: bootstrap anchors at startup and announce-triggered peers,
//! each opening a bidi stream bridged onto a byte-channel.

use std::sync::Arc;

use iroh::{Endpoint, EndpointAddr, EndpointId};
use leviculum_std::api::Node as LevNode;

use super::engine::ALPN;
use super::{Links, bridge};

/// Dial a configured bootstrap peer (an `EndpointId` string) to form an RNS
/// interface when other transports can't reach an anchor.
pub async fn bootstrap(endpoint: Endpoint, engine: Arc<LevNode>, links: Links, peer: String) {
    match peer.trim().parse::<EndpointId>() {
        Ok(id) => dial(endpoint, engine, links, EndpointAddr::from(id)).await,
        Err(e) => tracing::warn!(peer, error = %e, "invalid iroh bootstrap peer"),
    }
}

/// Open a connection to `addr` and bridge its bidi stream onto a byte-channel.
pub async fn dial(endpoint: Endpoint, engine: Arc<LevNode>, links: Links, addr: EndpointAddr) {
    match endpoint.connect(addr, ALPN).await {
        Ok(conn) => bridge::dial_link(&engine, &links, conn).await,
        Err(e) => tracing::warn!(error = %e, "iroh dial failed"),
    }
}
