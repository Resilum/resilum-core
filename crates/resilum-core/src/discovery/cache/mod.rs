//! Persistent per-service peer cache. Records hold `endpoint` as hex plus
//! `first_seen` / `last_seen` UNIX ts, so the next process can warm-start with
//! whoever was reachable last time.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

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
    ticker.tick().await; // consume the immediate first tick
    loop {
        ticker.tick().await;
        let now = now_ts();
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
pub struct Record {
    pub endpoint: String, // hex
    pub first_seen: f64,
    pub last_seen: f64,
}

pub fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub fn load(path: &Path) -> Vec<Record> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(path: &Path, records: &[Record]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(&serde_json::to_vec_pretty(records)?)?;
    f.sync_all()?;
    fs::rename(tmp, path)
}

/// Insert a fresh record or refresh `last_seen` on an existing match.
pub fn upsert(records: &mut Vec<Record>, endpoint: &[u8], now: f64) {
    let hex = hex_encode(endpoint);
    if let Some(rec) = records.iter_mut().find(|r| r.endpoint == hex) {
        rec.last_seen = now;
        return;
    }
    records.push(Record {
        endpoint: hex,
        first_seen: now,
        last_seen: now,
    });
}

/// Drop records whose `last_seen` is older than `ttl_seconds`. Returns how many
/// were removed.
pub fn prune(records: &mut Vec<Record>, ttl_seconds: f64, now: f64) -> usize {
    let cutoff = now - ttl_seconds;
    let before = records.len();
    records.retain(|r| r.last_seen >= cutoff);
    before - records.len()
}

/// The `n` most-recently-seen endpoints as raw bytes, most recent first.
pub fn top_n(records: &[Record], n: usize) -> Vec<Vec<u8>> {
    let mut ranked: Vec<&Record> = records.iter().collect();
    ranked.sort_by(|a, b| b.last_seen.total_cmp(&a.last_seen));
    ranked
        .into_iter()
        .take(n)
        .filter_map(|r| hex_decode(&r.endpoint))
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests;
