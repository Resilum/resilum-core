use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::DestinationHash;
use leviculum_std::driver::ReticulumNode;

pub async fn every(engine: Arc<ReticulumNode>, ours: DestinationHash, interval: Duration) {
    loop {
        if let Err(error) = engine.announce_destination(&ours, None).await {
            tracing::debug!(%error, "our own announce did not go out");
        }
        tokio::time::sleep(interval).await;
    }
}
