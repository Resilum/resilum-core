use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub peer: String,
    pub rngit: String,
    pub repos: Vec<String>,
    pub last_seen_unix: u64,
}

pub struct Registry {
    entries: Mutex<HashMap<String, Entry>>,
    persist_path: Option<PathBuf>,
}

impl Registry {
    pub fn new(persist_path: Option<PathBuf>) -> Self {
        let entries = persist_path
            .as_ref()
            .and_then(|p| load(p).ok())
            .unwrap_or_default();
        Self {
            entries: Mutex::new(entries),
            persist_path,
        }
    }

    pub fn upsert(&self, peer_hex: String, rngit: String, repos: Vec<String>) {
        let entry = Entry {
            peer: peer_hex.clone(),
            rngit,
            repos,
            last_seen_unix: now(),
        };
        self.entries.lock().unwrap().insert(peer_hex, entry);
        self.persist();
    }

    pub fn snapshot(&self) -> Vec<Entry> {
        self.entries.lock().unwrap().values().cloned().collect()
    }

    fn persist(&self) {
        let Some(path) = self.persist_path.as_ref() else {
            return;
        };
        let snap = self.snapshot();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&snap) {
            let _ = std::fs::write(path, bytes);
        }
    }
}

fn load(path: &Path) -> std::io::Result<HashMap<String, Entry>> {
    let bytes = std::fs::read(path)?;
    let list: Vec<Entry> = serde_json::from_slice(&bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(list.into_iter().map(|e| (e.peer.clone(), e)).collect())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
