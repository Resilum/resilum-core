use super::*;
use crate::registry::admit::{FILTERS_PER_REQUEST, MAX_SUBSCRIBERS};

mod batch;
mod cap;
mod persistence;
mod replay;

fn sub(created_at: i64, last_byte: u8) -> Subscription {
    let mut lxmf = [0u8; 16];
    lxmf[15] = last_byte;
    Subscription {
        pubkey: [7u8; 32],
        lxmf,
        created_at,
    }
}

fn nth(pubkey: u8, created_at: i64) -> Subscription {
    Subscription {
        pubkey: [pubkey; 32],
        lxmf: [0u8; 16],
        created_at,
    }
}

/// A directory of this process's own, so the file-backed tests do not read
/// each other's registries when the suite runs them in parallel.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "resilum-nostr-registry-{name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}
