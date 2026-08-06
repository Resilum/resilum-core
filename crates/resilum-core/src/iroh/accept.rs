//! Accept loop: each inbound iroh connection's first bidi stream becomes a
//! byte-channel, so peers dialing us over iroh join the mesh.

use std::sync::Arc;

use iroh::Endpoint;
use leviculum_std::api::Node as LevNode;

use super::{Links, bridge};

pub async fn run(endpoint: Endpoint, engine: Arc<LevNode>, links: Links) {
    while let Some(incoming) = endpoint.accept().await {
        let engine = engine.clone();
        let links = links.clone();
        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => bridge::accept_link(&engine, &links, conn).await,
                Err(e) => tracing::warn!(error = %e, "iroh inbound connection failed"),
            }
        });
    }
}
