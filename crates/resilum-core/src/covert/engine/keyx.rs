//! Per-link session key. The client generates a random 32-byte key, seals it
//! to the server's public identity; only the server unseals with its private
//! key. From that point every datagram is authenticated by HMAC-SHA256 with
//! this key.

use leviculum_std::api::Identity;

pub const SESSION_KEY_LEN: usize = 32;

pub fn new_session_key() -> [u8; SESSION_KEY_LEN] {
    use rand_core::RngCore;
    let mut key = [0u8; SESSION_KEY_LEN];
    rand_core::OsRng.fill_bytes(&mut key);
    key
}

pub fn seal(server: &Identity, key: &[u8]) -> Option<Vec<u8>> {
    server.encrypt(key, &mut rand_core::OsRng).ok()
}

pub fn unseal(own: &Identity, token: &[u8]) -> Option<Vec<u8>> {
    let out = own.decrypt(token).ok()?;
    (out.len() == SESSION_KEY_LEN).then_some(out)
}
