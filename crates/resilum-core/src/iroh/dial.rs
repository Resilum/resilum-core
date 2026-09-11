//! Outbound dials: bootstrap anchors at startup and announce-triggered peers,
//! each opening a bidi stream bridged onto a byte-channel.

use std::sync::Arc;

use iroh::{Endpoint, EndpointAddr, EndpointId};

use super::bridge;
use super::engine::ALPN;
use super::wiring::{NOT_NAMED_UNTIL_THEY_ANNOUNCE, Wiring};
use crate::coordinates::PeerId;

/// Dial a configured bootstrap peer (an `EndpointId` string) to form an RNS
/// interface when other transports can't reach an anchor.
pub async fn bootstrap(endpoint: Endpoint, wiring: Arc<Wiring>, peer: String) {
    match peer.trim().parse::<EndpointId>() {
        Ok(id) => {
            dial(
                endpoint,
                wiring,
                EndpointAddr::from(id),
                NOT_NAMED_UNTIL_THEY_ANNOUNCE,
            )
            .await;
        }
        Err(e) => tracing::warn!(peer, error = %e, "invalid iroh bootstrap peer"),
    }
}

/// Open a connection to `addr` and bridge its bidi stream onto a byte-channel.
pub async fn dial(
    endpoint: Endpoint,
    wiring: Arc<Wiring>,
    addr: EndpointAddr,
    announced_by: Option<PeerId>,
) {
    match endpoint.connect(addr, ALPN).await {
        Ok(conn) => bridge::dial_link(&wiring, conn, announced_by).await,
        Err(e) => tracing::warn!(error = %e, "iroh dial failed"),
    }
}
