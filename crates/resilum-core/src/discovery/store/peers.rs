use std::path::PathBuf;

use resilum_store::Document;

use super::records::{Record, prune, top_n, upsert};
use super::{TOP_N_ACTIVE, TTL_SECONDS};

type Records = Vec<Record>;

pub(crate) struct Peers {
    remembered: Document<Records>,
}

impl Peers {
    pub(crate) fn open(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self {
                remembered: Document::in_memory(Records::new()),
            };
        };
        Self {
            remembered: Document::open_or_start_empty(path, Records::new(), decode, encode),
        }
    }

    pub(crate) fn seen(&self, endpoint: &[u8], now: f64) {
        self.remembered
            .change(|remembered| upsert(remembered, endpoint, now));
    }

    pub(crate) fn forget_stale(&self, now: f64) {
        self.remembered
            .change(|remembered| prune(remembered, TTL_SECONDS, now));
    }

    pub(crate) fn most_recent(&self) -> Vec<Vec<u8>> {
        self.remembered
            .read(|remembered| top_n(remembered, TOP_N_ACTIVE))
    }
}

fn decode(bytes: &[u8]) -> Result<Records, String> {
    Ok(serde_json::from_slice(bytes).unwrap_or_default())
}

fn encode(remembered: &Records) -> Vec<u8> {
    serde_json::to_vec_pretty(remembered).unwrap_or_default()
}

#[cfg(test)]
#[path = "peers_tests.rs"]
mod tests;
