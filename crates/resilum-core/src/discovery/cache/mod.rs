//! Persistent per-service peer cache: who was reachable last time, so the next
//! process can warm-start against them instead of waiting for an announce.

use std::fs;
use std::io::Write;
use std::path::Path;

use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};

use crate::wall_clock::unix_now;

pub const TTL_SECONDS: f64 = 24.0 * 60.0 * 60.0;
pub const TOP_N_ACTIVE: usize = 10;
pub const PRUNE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);

pub fn path_for(storage_root: &Path, service: &str) -> std::path::PathBuf {
    storage_root
        .join("discovered")
        .join(format!("{service}.json"))
}

pub async fn run_prune_loop(storage_root: std::path::PathBuf, services: Vec<String>) {
    let mut ticker = tokio::time::interval(PRUNE_INTERVAL);
    // A cache loaded seconds ago has nothing worth pruning, and `warm_start`
    // has already pruned it once.
    ticker.tick().await;
    loop {
        ticker.tick().await;
        let now = unix_now();
        for service in &services {
            let path = path_for(&storage_root, service);
            let mut records = load(&path);
            if prune(&mut records, TTL_SECONDS, now) > 0 {
                let _ = save(&path, &records);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Record {
    pub(super) endpoint_hex: String,
    pub(super) first_seen: f64,
    pub(super) last_seen: f64,
}

/// Anything unreadable reads as no peers, never as an error: this is a
/// warm-start hint, and a node that refused to start over a stale or corrupt
/// hint would be trading a working start for a faster one.
pub(super) fn load(path: &Path) -> Vec<Record> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub(super) fn save(path: &Path, records: &[Record]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(&serde_json::to_vec_pretty(records)?)?;
    f.sync_all()?;
    fs::rename(tmp, path)
}

pub(super) fn upsert(records: &mut Vec<Record>, endpoint: &[u8], now: f64) {
    let endpoint_hex = HEXLOWER.encode(endpoint);
    if let Some(rec) = records.iter_mut().find(|r| r.endpoint_hex == endpoint_hex) {
        rec.last_seen = now;
        return;
    }
    records.push(Record {
        endpoint_hex,
        first_seen: now,
        last_seen: now,
    });
}

pub(super) fn prune(records: &mut Vec<Record>, ttl_seconds: f64, now: f64) -> usize {
    let cutoff = now - ttl_seconds;
    let before = records.len();
    records.retain(|r| r.last_seen >= cutoff);
    before - records.len()
}

/// The `n` most-recently-seen endpoints as raw bytes, most recent first.
pub(super) fn top_n(records: &[Record], n: usize) -> Vec<Vec<u8>> {
    let mut ranked: Vec<&Record> = records.iter().collect();
    ranked.sort_by(|a, b| b.last_seen.total_cmp(&a.last_seen));
    ranked
        .into_iter()
        .take(n)
        .filter_map(|r| HEXLOWER.decode(r.endpoint_hex.as_bytes()).ok())
        .collect()
}

#[cfg(test)]
mod tests;
