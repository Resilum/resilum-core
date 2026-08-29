//! Payload markers that tell the tunnel apart from ordinary pings, since an
//! unprivileged client cannot choose the kernel-owned ICMP id. Request and
//! reply differ so the kernel's own echo of a request cannot pass for the
//! server's reply — the client only accepts the reply marker.

use sha2::{Digest, Sha256};

pub const MARKER_LEN: usize = 4;

pub fn request_marker(server_pubkey: &[u8]) -> [u8; MARKER_LEN] {
    marker(b"resilum-covert-icmp-request", server_pubkey)
}

pub fn reply_marker(server_pubkey: &[u8]) -> [u8; MARKER_LEN] {
    marker(b"resilum-covert-icmp-reply", server_pubkey)
}

fn marker(domain: &[u8], server_pubkey: &[u8]) -> [u8; MARKER_LEN] {
    let mut h = Sha256::new();
    h.update(domain);
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
    fn each_direction_and_key_gets_its_own_marker() {
        assert_eq!(request_marker(b"pk1"), request_marker(b"pk1"));
        assert_ne!(request_marker(b"pk1"), request_marker(b"pk2"));
        assert_ne!(request_marker(b"pk1"), reply_marker(b"pk1"));
    }
}
