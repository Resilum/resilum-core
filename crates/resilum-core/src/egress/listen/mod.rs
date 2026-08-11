//! Egress listen side: register a destination per service, announce each, and
//! forward every inbound link to its service's local TCP endpoint.

mod session;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{Destination, DestinationHash, DestinationType, Direction, Identity};
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

use crate::config::EgressListen;
use crate::link::Inbound;

#[derive(Clone)]
enum Backend {
    External(String),
    EmbeddedSocks,
}

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
    engine: Arc<ReticulumNode>,
    identity: Identity,
    services: Vec<EgressListen>,
    mut inbound: UnboundedReceiver<Inbound>,
) {
    let mut backends: HashMap<[u8; 16], Backend> = HashMap::new();
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
        let backend = match cfg.target.clone() {
            Some(t) => Backend::External(t),
            None => Backend::EmbeddedSocks,
        };
        backends.insert(*dest_hash.as_bytes(), backend);
    }

    while let Some((link_id, dest_hash, from_link)) = inbound.recv().await {
        if let Some(backend) = backends.get(dest_hash.as_bytes()) {
            let handle = engine.link_handle(&link_id);
            match backend.clone() {
                Backend::External(target) => {
                    tokio::spawn(session::session_external(handle, from_link, target));
                }
                Backend::EmbeddedSocks => {
                    tokio::spawn(session::session_embedded(handle, from_link));
                }
            }
        }
    }
    for announcer in announcers {
        announcer.abort();
    }
}

fn announce_loop(
    engine: Arc<ReticulumNode>,
    dest_hash: DestinationHash,
    interval: Duration,
    payload: Vec<u8>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match engine
                .announce_destination(&dest_hash, Some(&payload))
                .await
            {
                Ok(()) => tracing::debug!(
                    dest = ?data_encoding::HEXLOWER.encode(dest_hash.as_bytes()),
                    "egress announced",
                ),
                Err(e) => tracing::warn!(error = %e, "egress announce failed"),
            }
            tokio::time::sleep(interval).await;
        }
    })
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
