//! Adaptive per-interface announce cap: idle links let announces converge the
//! topology fast; once real traffic starts we drop the share to 1 % so data
//! has the channel.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use leviculum_std::InterfaceId;
use leviculum_std::api::Node as LevNode;

pub const CAP_IDLE_PERCENT: u32 = 100;
pub const CAP_BUSY_PERCENT: u32 = 1;
const THRESHOLD_BPS: u64 = 512;
const WINDOW: Duration = Duration::from_secs(2);
const COOLDOWN: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Entry {
    last_bytes: u64,
    busy_until: Option<Instant>,
}

pub struct CapController {
    engine: Arc<LevNode>,
    entries: Mutex<HashMap<InterfaceId, Entry>>,
}

impl CapController {
    pub fn new(engine: Arc<LevNode>) -> Arc<Self> {
        Arc::new(Self {
            engine,
            entries: Mutex::new(HashMap::new()),
        })
    }

    /// Start tracking an interface; set the cap to IDLE at once.
    pub fn attach(&self, id: InterfaceId) {
        self.entries
            .lock()
            .expect("cap entries")
            .entry(id)
            .or_default();
        self.engine.set_interface_announce_cap(id, CAP_IDLE_PERCENT);
    }

    pub fn detach(&self, id: InterfaceId) {
        self.entries.lock().expect("cap entries").remove(&id);
    }
}

/// Periodic tick loop: adjust each tracked interface's cap by throughput.
pub async fn run(controller: Arc<CapController>) {
    let mut ticker = tokio::time::interval(WINDOW);
    ticker.tick().await;
    loop {
        ticker.tick().await;
        let by_id: HashMap<InterfaceId, (u64, u64)> = controller
            .engine
            .interface_stats()
            .into_iter()
            .map(|s| (s.interface_id, (s.rx_bytes, s.tx_bytes)))
            .collect();
        let now = Instant::now();
        let mut entries = controller.entries.lock().expect("cap entries");
        for (id, entry) in entries.iter_mut() {
            let Some(&(rx, tx)) = by_id.get(id) else {
                continue;
            };
            let total = rx.saturating_add(tx);
            let delta = total.saturating_sub(entry.last_bytes);
            entry.last_bytes = total;
            let bps = delta / WINDOW.as_secs();
            if bps > THRESHOLD_BPS {
                entry.busy_until = Some(now + COOLDOWN);
                controller
                    .engine
                    .set_interface_announce_cap(*id, CAP_BUSY_PERCENT);
            } else if entry.busy_until.map(|t| now >= t).unwrap_or(true) {
                entry.busy_until = None;
                controller
                    .engine
                    .set_interface_announce_cap(*id, CAP_IDLE_PERCENT);
            }
        }
    }
}
