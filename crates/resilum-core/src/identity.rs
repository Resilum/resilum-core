//! Load-or-create the node identity, persisted as raw private-key bytes under
//! the storage path so destination hashes survive restarts.

use std::path::Path;

use leviculum_std::api::{self, Identity};

const FILE: &str = "identity";

/// Load the identity from `<dir>/identity`, or generate and persist one. Returns
/// a fresh unpersisted identity when the file cannot be read or written.
pub(crate) fn load_or_create(dir: &Path) -> Identity {
    let path = dir.join(FILE);
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(identity) = Identity::from_private_key_bytes(&bytes)
    {
        return identity;
    }
    let identity = api::generate_identity();
    persist(&path, &identity);
    identity
}

fn persist(path: &Path, identity: &Identity) {
    let Ok(bytes) = identity.private_key_bytes() else {
        return;
    };
    if std::fs::write(path, bytes).is_ok() {
        restrict(path);
    }
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
    fn reloads_the_same_identity() {
        let dir = std::env::temp_dir().join(format!("resilum-id-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let first = load_or_create(&dir);
        let second = load_or_create(&dir);
        assert_eq!(first.hash(), second.hash(), "second load reused the file");
        assert!(dir.join(FILE).exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
