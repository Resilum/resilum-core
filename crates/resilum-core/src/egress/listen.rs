//! Egress listen side: register a destination per service, announce each, and
//! forward every inbound link to its service's local TCP endpoint.

use std::collections::HashMap;
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

/// The destination hash this node announces for `service`, for self-skip.
pub(crate) fn dest_hash(identity: Identity, service: &str) -> Vec<u8> {
    build_destination(identity, service)
        .hash()
        .as_bytes()
        .to_vec()
}

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

/// Register and announce one destination per service, then forward each inbound
/// link to its service's target until the router's `inbound` closes.
pub async fn run(
    engine: Arc<LevNode>,
    identity: Identity,
    services: Vec<EgressListen>,
    mut inbound: UnboundedReceiver<Inbound>,
) {
    let mut targets: HashMap<[u8; 16], String> = HashMap::new();
    let mut announcers = Vec::new();
    for cfg in &services {
        let dest = build_destination(identity.clone(), &cfg.service);
        let dest_hash = *dest.hash();
        engine.register_destination(dest);
        let payload = crate::announce_payload::pack(None, &cfg.exit_country, &[]);
        announcers.push(announce_loop(
            engine.clone(),
            dest_hash,
            cfg.announce_interval,
            payload,
        ));
        targets.insert(*dest_hash.as_bytes(), cfg.target.clone());
    }

    while let Some((link_id, dest_hash, from_link)) = inbound.recv().await {
        if let Some(target) = targets.get(dest_hash.as_bytes()) {
            let handle = engine.accept_link(&link_id);
            tokio::spawn(session(handle, from_link, target.clone()));
        }
    }
    for announcer in announcers {
        announcer.abort();
    }
}

fn announce_loop(
    engine: Arc<LevNode>,
    dest_hash: DestinationHash,
    interval: Duration,
    payload: Vec<u8>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let _ = engine.announce(&dest_hash, Some(&payload)).await;
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
