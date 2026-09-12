mod hostname;
mod peers;
mod records;

pub(crate) use hostname::Advertised;
#[cfg(all(unix, feature = "ygg"))]
pub(crate) use hostname::say_the_address_is;
pub(crate) use peers::Peers;

use std::path::Path;
use std::sync::Arc;

use crate::wall_clock::unix_now;

pub const TTL_SECONDS: f64 = 24.0 * 60.0 * 60.0;
pub const TOP_N_ACTIVE: usize = 10;
pub const PRUNE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);

pub fn path_for(storage_root: &Path, service: &str) -> std::path::PathBuf {
    storage_root
        .join("discovered")
        .join(format!("{service}.json"))
}

pub async fn run_prune_loop(plugins: Arc<super::Discovery>) {
    let mut ticker = tokio::time::interval(PRUNE_INTERVAL);
    warm_start_already_pruned_it(&mut ticker).await;
    loop {
        ticker.tick().await;
        plugins.forget_stale_peers(unix_now());
    }
}

async fn warm_start_already_pruned_it(ticker: &mut tokio::time::Interval) {
    ticker.tick().await;
}
