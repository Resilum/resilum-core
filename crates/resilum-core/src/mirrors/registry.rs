use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use resilum_store::Document;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub peer: String,
    pub rngit: String,
    pub repos: Vec<String>,
    pub last_seen_unix: u64,
}

type Kept = HashMap<String, Entry>;

pub struct Registry {
    entries: Document<Kept>,
}

impl Registry {
    #[must_use]
    pub fn new(persist_path: Option<PathBuf>) -> Self {
        let Some(path) = persist_path else {
            return Self {
                entries: Document::in_memory(Kept::new()),
            };
        };
        Self {
            entries: Document::open_or_start_empty(path, Kept::new(), decode, encode),
        }
    }

    pub fn upsert(&self, peer_hex: String, rngit: String, repos: Vec<String>) {
        let entry = Entry {
            peer: peer_hex.clone(),
            rngit,
            repos,
            last_seen_unix: now(),
        };
        self.entries
            .change(|entries| entries.insert(peer_hex, entry));
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<Entry> {
        self.entries
            .read(|entries| entries.values().cloned().collect())
    }
}

fn decode(bytes: &[u8]) -> Result<Kept, String> {
    let listed: Vec<Entry> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    Ok(listed.into_iter().map(|e| (e.peer.clone(), e)).collect())
}

fn encode(entries: &Kept) -> Vec<u8> {
    let listed: Vec<&Entry> = entries.values().collect();
    serde_json::to_vec_pretty(&listed).unwrap_or_default()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
