//! Per-server ICMP echo id. Both endpoints derive it from the server's public
//! key — the client from the announced key, the server from its own — so the
//! echo-reply suppression rule and the carrier agree on a marker without any
//! hard-coded constant.

use sha2::{Digest, Sha256};

const DOMAIN: &[u8] = b"resilum-covert-icmp-id";

pub fn tunnel_id(server_pubkey: &[u8]) -> u16 {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update(server_pubkey);
    let d = h.finalize();
    let n = u16::from_be_bytes([d[0], d[1]]);
    if n == 0 { 1 } else { n }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_nonzero() {
        assert_eq!(tunnel_id(b"pubkey1"), tunnel_id(b"pubkey1"));
        assert_ne!(tunnel_id(b"pubkey1"), tunnel_id(b"pubkey2"));
        // No zero ids: 0 collides with kernel-managed echo sockets which pick
        // their own id and read it back as 0 on some paths.
        for k in [b"a".as_slice(), b"", b"\0\0\0\0"] {
            assert_ne!(tunnel_id(k), 0);
        }
    }
}
