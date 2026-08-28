//! Per-server tunnel marker: the first bytes of every echo payload both
//! endpoints derive from the server's public key. An unprivileged client
//! cannot choose the ICMP id — the kernel owns it — so the tunnel is told
//! apart from ordinary pings by this marker in the payload, not by the id.

use sha2::{Digest, Sha256};

pub const MARKER_LEN: usize = 4;

const DOMAIN: &[u8] = b"resilum-covert-icmp-marker";

pub fn tunnel_marker(server_pubkey: &[u8]) -> [u8; MARKER_LEN] {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update(server_pubkey);
    let d = h.finalize();
    let mut marker = [0u8; MARKER_LEN];
    marker.copy_from_slice(&d[..MARKER_LEN]);
    marker
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_key_bound() {
        assert_eq!(tunnel_marker(b"pubkey1"), tunnel_marker(b"pubkey1"));
        assert_ne!(tunnel_marker(b"pubkey1"), tunnel_marker(b"pubkey2"));
    }
}
