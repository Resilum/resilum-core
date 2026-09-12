use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::{Destination, DestinationHash, DestinationType, Direction, Identity};
use leviculum_std::driver::ReticulumNode;

use super::handover::{SharedWithRngit, read_rngit_destination};
use super::payload::{Advert, pack};
use super::registry::Registry;
use super::{APP_NAME, ASPECT};

pub async fn run_produce(
    engine: Arc<ReticulumNode>,
    identity: Identity,
    interval: Duration,
    advertised_repos: Vec<String>,
    rngit_destination_file: PathBuf,
    known: Arc<Registry>,
) {
    let dest_hash = register(&engine, identity);
    let shared = Arc::new(SharedWithRngit::beside(&rngit_destination_file));
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let Some(advert) = trade_lists_with_rngit(
            shared.clone(),
            rngit_destination_file.clone(),
            advertised_repos.clone(),
            known.snapshot(),
        )
        .await
        else {
            continue;
        };
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

async fn trade_lists_with_rngit(
    shared: Arc<SharedWithRngit>,
    destination_file: PathBuf,
    advertised_repos: Vec<String>,
    known: Vec<super::Entry>,
) -> Option<Advert> {
    let traded = tokio::task::spawn_blocking(move || {
        let rngit_dest = read_rngit_destination(&destination_file).or_else(|| {
            tracing::debug!(
                path = %destination_file.display(),
                "rngit destination not ready, skipping mirror advert",
            );
            None
        })?;
        shared.ask_for_what_we_lack(&advertised_repos, &known);
        let ours = shared.of_these_we_serve(&advertised_repos);
        if ours.is_empty() {
            tracing::debug!("no mirror is served here yet, nothing to announce");
            return None;
        }
        Some(Advert::new(rngit_dest, ours))
    })
    .await;
    match traded {
        Ok(advert) => advert,
        Err(e) => {
            tracing::warn!(error = %e, "the mirror lists could not be read");
            None
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
