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

/// The `lxmf.delivery` destination hash this identity answers on — the address
/// peers put in a message, and the `source_hash` this node signs one with.
///
/// Computed rather than read off a registered destination, so it is known
/// before the node starts.
pub fn lxmf_address(identity: &Identity) -> [u8; 16] {
    let name_hash = Destination::compute_name_hash("lxmf", &["delivery"]);
    *Destination::compute_destination_hash(&name_hash, identity.hash()).as_bytes()
}

pub fn lxmf_address_hex(identity: &Identity) -> String {
    HEXLOWER.encode(&lxmf_address(identity))
}

pub fn load_or_create(dir: &Path) -> Identity {
    load_or_create_at(&dir.join(FILE))
}

/// Loads `path`, else generates and persists a fresh identity at `0600`.
pub fn load_or_create_at(path: &Path) -> Identity {
    if let Ok(bytes) = resilum_store::read_bytes(path)
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
    if resilum_store::write_bytes(path, &bytes).is_ok()
        && let Err(e) = resilum_store::own_eyes_only(path)
    {
        tracing::warn!(path = %path.display(), error = %e, "the identity file is readable by others");
    }
}

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
        let dir = tempfile::tempdir().expect("a temporary directory");

        let first = load_or_create(dir.path());
        let second = load_or_create(dir.path());
        assert_eq!(first.hash(), second.hash(), "second load reused the file");
        assert!(dir.path().join(FILE).exists());
    }
}
