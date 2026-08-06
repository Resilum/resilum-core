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
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice())
    {
        return SecretKey::from_bytes(&arr);
    }
    let key = SecretKey::generate();
    if std::fs::write(&path, key.to_bytes()).is_ok() {
        restrict(&path);
    } else {
        tracing::warn!(path = %path.display(), "persisting iroh secret failed");
    }
    key
}

#[cfg(unix)]
fn restrict(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_reloads_the_same_key() {
        let dir = std::env::temp_dir().join(format!("resilum-iroh-key-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = load_or_create(&dir).public();
        let second = load_or_create(&dir).public();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(first, second);
    }
}
