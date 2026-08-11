use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{Destination, DestinationHash, DestinationType, Direction, Identity};
use leviculum_std::driver::ReticulumNode;

use super::payload::{Advert, pack};
use super::{APP_NAME, ASPECT};

pub async fn run_produce(
    engine: Arc<ReticulumNode>,
    identity: Identity,
    interval: Duration,
    advertised_repos: Vec<String>,
    rngit_destination_file: PathBuf,
) {
    let dest_hash = register(&engine, identity);
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let Some(rngit_dest) = read_rngit_destination(&rngit_destination_file) else {
            tracing::debug!(
                path = %rngit_destination_file.display(),
                "rngit destination not ready, skipping mirror advert",
            );
            continue;
        };
        let advert = Advert::new(rngit_dest, advertised_repos.clone());
        match engine
            .announce_destination(&dest_hash, Some(&pack(&advert)))
            .await
        {
            Ok(()) => tracing::debug!(
                repos = ?advert.repos,
                rngit = %advert.rngit,
                "mirror advert announced",
            ),
            Err(e) => tracing::warn!(error = %e, "mirror advert failed"),
        }
    }
}

fn register(engine: &ReticulumNode, identity: Identity) -> DestinationHash {
    let dest = Destination::new(
        Some(identity),
        Direction::In,
        DestinationType::Single,
        APP_NAME,
        ASPECT,
    )
    .expect("IN/SINGLE destination with an identity is always valid");
    let hash = *dest.hash();
    engine.register_destination(dest);
    hash
}

fn read_rngit_destination(path: &PathBuf) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    (trimmed.len() == 32 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| trimmed.to_owned())
}
