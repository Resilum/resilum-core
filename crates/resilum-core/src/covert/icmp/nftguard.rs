//! Suppress the kernel's echo-reply for our tunnel packets only, so a passive
//! observer sees one reply per request instead of two — the kernel's and our
//! crafted one. The rule matches our marker in the echo payload, so every
//! other ping is still answered normally.
//!
//! `AF_PACKET` taps below netfilter, so our sniffer still sees the dropped
//! requests. Linux only.

use std::process::Command;

use super::marker::MARKER_LEN;

const TABLE: &str = "resilum_covert";
const MARKER_OFFSET_BITS: usize = 64; // past the 8-byte ICMP header

pub fn install(marker: [u8; MARKER_LEN]) -> bool {
    remove();
    let ok = nft(&["add", "table", "inet", TABLE])
        && nft(&[
            "add",
            "chain",
            "inet",
            TABLE,
            "input",
            "{ type filter hook input priority 0; policy accept; }",
        ])
        && drop_rule("icmp", marker)
        && drop_rule("icmpv6", marker);
    if !ok {
        remove();
    }
    ok
}

pub fn remove() {
    let _ = nft(&["delete", "table", "inet", TABLE]);
}

fn drop_rule(proto: &str, marker: [u8; MARKER_LEN]) -> bool {
    let field = format!("@th,{MARKER_OFFSET_BITS},{}", MARKER_LEN * 8);
    let value = format!("0x{:08x}", u32::from_be_bytes(marker));
    nft(&[
        "add",
        "rule",
        "inet",
        TABLE,
        "input",
        proto,
        "type",
        "echo-request",
        &field,
        &value,
        "drop",
    ])
}

fn nft(args: &[&str]) -> bool {
    Command::new("nft")
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// RAII wrapper: installs the guard on construction, removes it on drop.
pub struct Guard {
    installed: bool,
}

impl Guard {
    pub fn install(marker: [u8; MARKER_LEN]) -> Self {
        Self {
            installed: install(marker),
        }
    }

    pub fn is_installed(&self) -> bool {
        self.installed
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if self.installed {
            remove();
        }
    }
}
