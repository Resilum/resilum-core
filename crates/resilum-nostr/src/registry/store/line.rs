//! The on-disk shape of one record.

use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};

use super::Record;
use crate::event::decode_hex;
use crate::registry::BatchId;

#[derive(Serialize, Deserialize)]
pub(super) struct Line {
    pubkey: String,
    lxmf: String,
    created_at: i64,
    last_seen: i64,
    /// Absent on a line a version before batching wrote.
    #[serde(default)]
    batch: Option<usize>,
}

/// A decoded line, batch and all: resolving a missing batch has to see every
/// record on file at once, so decoding stays separate from that decision.
pub(super) struct Loaded {
    pub(super) pubkey: [u8; 32],
    pub(super) lxmf: [u8; 16],
    pub(super) created_at: i64,
    pub(super) last_seen: i64,
    pub(super) batch: Option<BatchId>,
}

pub(super) fn decode(text: &str) -> Result<Loaded, String> {
    let parsed: Line = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(Loaded {
        pubkey: decode_hex::<32>(&parsed.pubkey)?,
        lxmf: decode_hex::<16>(&parsed.lxmf)?,
        created_at: parsed.created_at,
        last_seen: parsed.last_seen,
        batch: parsed.batch.map(BatchId::from_stored),
    })
}

pub(super) fn encode(pubkey: &[u8; 32], record: &Record) -> Line {
    Line {
        pubkey: HEXLOWER.encode(pubkey),
        lxmf: HEXLOWER.encode(&record.lxmf),
        created_at: record.created_at,
        last_seen: record.last_seen,
        batch: Some(record.batch.stored()),
    }
}
