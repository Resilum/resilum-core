//! Accept loop: each inbound iroh connection's first bidi stream becomes a
//! byte-channel, so peers dialing us over iroh join the mesh.

use std::sync::Arc;

use iroh::Endpoint;

use super::bridge;
use super::wiring::Wiring;

pub async fn run(endpoint: Endpoint, wiring: Arc<Wiring>, arriving: Arc<resilum_tasks::Nursery>) {
    while let Some(incoming) = endpoint.accept().await {
        let wiring = wiring.clone();
        arriving.keep("iroh: one inbound connection", async move {
            match incoming.await {
                Ok(conn) => bridge::accept_link(&wiring, conn).await,
                Err(e) => tracing::warn!(error = %e, "iroh inbound connection failed"),
            }
        });
    }
}
