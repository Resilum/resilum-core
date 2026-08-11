//! Poll the interface list; fire `trigger` when a new interface appears so the
//! discovery produce loop re-announces without waiting a full interval.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::InterfaceId;
use leviculum_std::driver::ReticulumNode;
use tokio::sync::Notify;

const WATCH_INTERVAL: Duration = Duration::from_secs(5);

pub async fn run(engine: Arc<ReticulumNode>, trigger: Arc<Notify>) {
    let mut seen: HashSet<InterfaceId> = HashSet::new();
    let mut first_observation = true;
    let mut ticker = tokio::time::interval(WATCH_INTERVAL);
    loop {
        ticker.tick().await;
        let current: HashSet<InterfaceId> = engine
            .interface_stats()
            .into_iter()
            .map(|s| s.interface_id)
            .collect();
        if !first_observation && !current.is_subset(&seen) {
            trigger.notify_waiters();
        }
        seen = current;
        first_observation = false;
    }
}
