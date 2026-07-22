//! Suppress the kernel's echo-reply for our tunnel packets only. Without this
//! a covert server answers each request twice — once from the kernel, once
//! with our crafted reply — doubling an already narrow channel. The rule
//! matches this server's echo id, so every other ping is still answered
//! normally.
//!
//! `AF_PACKET` taps below netfilter, so our sniffer still sees the dropped
//! requests. Linux only.

use std::process::Command;

const TABLE: &str = "resilum_covert";

/// Install the nft table + chain + drop rule for `ident`. Returns `true` when
/// every step succeeded; a partial install is torn down before returning.
///
/// On success, callers should call [`remove`] on shutdown. A dropped `Guard`
/// (see [`Guard::install`]) does that automatically.
pub fn install(ident: u16) -> bool {
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
        && drop_rule("icmp", ident)
        && drop_rule("icmpv6", ident);
    if !ok {
        remove();
    }
    ok
}

pub fn remove() {
    let _ = nft(&["delete", "table", "inet", TABLE]);
}

fn drop_rule(proto: &str, ident: u16) -> bool {
    let id = ident.to_string();
    nft(&[
        "add",
        "rule",
        "inet",
        TABLE,
        "input",
        proto,
        "type",
        "echo-request",
        proto,
        "id",
        &id,
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
    pub fn install(ident: u16) -> Self {
        Self {
            installed: install(ident),
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
