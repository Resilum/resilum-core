use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::store::{Kept, whatever_the_last_run_left, write_atomically};
use crate::storage::Writer;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub peer: String,
    pub rngit: String,
    pub repos: Vec<String>,
    pub last_seen_unix: u64,
}

pub struct Registry {
    entries: Mutex<Kept>,
    writer: Writer<Kept>,
}

impl Registry {
    #[must_use]
    pub fn new(persist_path: Option<PathBuf>) -> Self {
        let Some(path) = persist_path else {
            return Self {
                entries: Mutex::new(HashMap::new()),
                writer: Writer::nowhere_to_write(),
            };
        };
        let entries = whatever_the_last_run_left(&path);
        Self {
            writer: Writer::spawn(path, entries.clone(), replace, write_atomically),
            entries: Mutex::new(entries),
        }
    }

    pub fn upsert(&self, peer_hex: String, rngit: String, repos: Vec<String>) {
        let entry = Entry {
            peer: peer_hex.clone(),
            rngit,
            repos,
            last_seen_unix: now(),
        };
        let mut entries = self.lock();
        entries.insert(peer_hex, entry);
        self.writer.send(entries.clone());
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<Entry> {
        self.lock().values().cloned().collect()
    }

    fn lock(&self) -> MutexGuard<'_, Kept> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn replace(held: &mut Kept, latest: Kept) {
    *held = latest;
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
