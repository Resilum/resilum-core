use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Record {
    pub(crate) endpoint_hex: String,
    pub(crate) first_seen: f64,
    pub(crate) last_seen: f64,
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
#[path = "records_tests.rs"]
mod tests;
