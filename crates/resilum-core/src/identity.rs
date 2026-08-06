//! Persist the node identity so destination hashes survive restarts.

use std::path::Path;

use data_encoding::{BASE64, HEXLOWER};
use leviculum_std::Destination;
use leviculum_std::api::{self, Identity};

const FILE: &str = "identity";

pub fn generate() -> Identity {
    api::generate_identity()
}

pub fn from_base64(private_base64: &str) -> Option<Identity> {
    let bytes = BASE64.decode(private_base64.as_bytes()).ok()?;
    Identity::from_private_key_bytes(&bytes).ok()
}

pub fn to_base64(identity: &Identity) -> Option<String> {
    Some(BASE64.encode(&identity.private_key_bytes().ok()?))
}

pub fn identity_hash_hex(identity: &Identity) -> String {
    HEXLOWER.encode(identity.hash())
}

pub fn lxmf_address_hex(identity: &Identity) -> String {
    let name_hash = Destination::compute_name_hash("lxmf", &["delivery"]);
    let dest = Destination::compute_destination_hash(&name_hash, identity.hash());
    HEXLOWER.encode(dest.as_bytes())
}

pub fn load_or_create(dir: &Path) -> Identity {
    load_or_create_at(&dir.join(FILE))
}

/// Loads `path`, else generates and persists a fresh identity at `0600`.
pub fn load_or_create_at(path: &Path) -> Identity {
    if let Ok(bytes) = std::fs::read(path)
        && let Ok(identity) = Identity::from_private_key_bytes(&bytes)
    {
        return identity;
    }
    let identity = api::generate_identity();
    persist(path, &identity);
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
    fn blob_roundtrips_and_derives_stable_hashes() {
        let id = generate();
        let b64 = to_base64(&id).expect("private key");
        let reloaded = from_base64(&b64).expect("valid blob");
        assert_eq!(identity_hash_hex(&id), identity_hash_hex(&reloaded));
        assert_eq!(lxmf_address_hex(&id), lxmf_address_hex(&reloaded));
        assert_eq!(identity_hash_hex(&id).len(), 32, "identity hash is hex16");
        assert_eq!(lxmf_address_hex(&id).len(), 32, "lxmf address is hex16");
        assert_ne!(identity_hash_hex(&id), lxmf_address_hex(&id));
        assert!(from_base64("not valid base64 !!").is_none());
    }

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
