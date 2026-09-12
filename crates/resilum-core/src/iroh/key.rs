//! The node's persisted iroh secret. A dedicated ed25519 key, never the RNS
//! identity, so the two protocols share no key material; the resulting
//! `EndpointId` reaches peers over announces and config, never typed by hand.

use std::path::Path;

use iroh::SecretKey;

const KEY_FILE: &str = "iroh_secret";

/// Load the secret from `dir/iroh_secret`, creating and persisting a fresh one
/// when it is absent or unreadable.
pub fn load_or_create(dir: &Path) -> SecretKey {
    let path = dir.join(KEY_FILE);
    if let Ok(bytes) = resilum_store::read_bytes(&path)
        && let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice())
    {
        return SecretKey::from_bytes(&arr);
    }
    let key = SecretKey::generate();
    match resilum_store::write_bytes(&path, &key.to_bytes()) {
        Ok(()) => {
            if let Err(e) = resilum_store::own_eyes_only(&path) {
                tracing::warn!(path = %path.display(), error = %e, "the iroh secret is readable by others");
            }
        }
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "persisting iroh secret failed")
        }
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_reloads_the_same_key() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        let first = load_or_create(dir.path()).public();
        let second = load_or_create(dir.path()).public();

        assert_eq!(first, second);
    }
}
