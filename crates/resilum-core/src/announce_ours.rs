use std::sync::Arc;
use std::time::Duration;

use leviculum_std::api::DestinationHash;
use leviculum_std::driver::ReticulumNode;

pub async fn every(engine: Arc<ReticulumNode>, ours: DestinationHash, interval: Duration) {
    loop {
        let _ = engine.announce_destination(&ours, None).await;
        tokio::time::sleep(interval).await;
    }
}
