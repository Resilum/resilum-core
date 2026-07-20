//! Egress listen side: register a destination, announce it, and forward each
//! inbound link to a fixed local TCP endpoint.

use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{
    Destination, DestinationHash, DestinationType, Direction, Identity, LinkHandle, Node as LevNode,
};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::task::JoinHandle;

use crate::config::EgressListen;
use crate::link::{Inbound, LinkMsg};
use crate::pump::pump;

const APP_NAME: &str = "resilum";

fn build_destination(identity: Identity, service: &str) -> Destination {
    Destination::new(
        Some(identity),
        Direction::In,
        DestinationType::Single,
        APP_NAME,
        &["bridge", "tcp", service],
    )
    .expect("IN/SINGLE destination with an identity is always valid")
}

/// Register the egress destination and forward inbound links to `cfg.target`
/// until the router's `inbound` closes.
pub async fn run(
    engine: Arc<LevNode>,
    identity: Identity,
    cfg: EgressListen,
    mut inbound: UnboundedReceiver<Inbound>,
) {
    let dest = build_destination(identity, &cfg.service);
    let dest_hash = *dest.hash();
    engine.register_destination(dest);
    let announcer = announce_loop(engine.clone(), dest_hash, cfg.announce_interval);

    while let Some((link_id, from_link)) = inbound.recv().await {
        let handle = engine.accept_link(&link_id);
        tokio::spawn(session(handle, from_link, cfg.target.clone()));
    }
    announcer.abort();
}

fn announce_loop(
    engine: Arc<LevNode>,
    dest_hash: DestinationHash,
    interval: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let _ = engine.announce(&dest_hash, None).await;
            tokio::time::sleep(interval).await;
        }
    })
}

async fn session(mut handle: LinkHandle, from_link: UnboundedReceiver<LinkMsg>, target: String) {
    let tcp = match TcpStream::connect(&target).await {
        Ok(tcp) => tcp,
        Err(_) => {
            let _ = handle.close().await;
            return;
        }
    };
    let (to_link, mut to_link_rx) = mpsc::unbounded_channel();
    let pumping = tokio::spawn(pump(tcp, from_link, to_link));
    while let Some(bytes) = to_link_rx.recv().await {
        if handle.send(&bytes).await.is_err() {
            break;
        }
    }
    let _ = pumping.await;
    let _ = handle.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use leviculum_std::api::generate_identity;

    #[test]
    fn destination_hash_is_stable_per_service() {
        let id = generate_identity();
        let a = *build_destination(id.clone(), "ygg").hash();
        let b = *build_destination(id.clone(), "ygg").hash();
        let c = *build_destination(id, "tor").hash();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
