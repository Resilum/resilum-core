//! Nostr bridge configuration: upstreams, admission control, and retention.
//!
//! An empty `allow_npubs` makes this a public bridge, open to any subscriber.
//! A populated `allow_npubs` makes this a personal bridge that only the
//! listed npubs may use.

use serde::Deserialize;
use std::time::Duration;

/// Unknown keys are refused rather than ignored: a misspelled `allow_npubs`
/// would otherwise leave the list empty, and an empty list admits everyone.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NostrConfig {
    #[serde(default)]
    pub upstreams: Vec<String>,
    /// Whether to advertise this bridge as the `nostr_relay` service in the
    /// discovery announce; it changes nothing about what the bridge carries.
    #[serde(default)]
    pub publish: bool,
    #[serde(default)]
    pub allow_npubs: Vec<String>,
    /// Whether to also take kind-4 direct messages, which NIP-17 replaced.
    ///
    /// Deliberately one-sided: it widens what the bridge accepts inbound and
    /// leaves `publish_kinds` alone, so a subscriber whose client still signs
    /// kind 4 can publish through a bridge that will not carry kind 4 back to
    /// it.
    #[serde(default = "default_legacy_dm")]
    pub legacy_dm: bool,
    #[serde(default = "default_publish_kinds")]
    pub publish_kinds: Vec<u32>,
    /// One window governing three lifetimes: how long a subscription stays
    /// live without a refresh, how long an undelivered event is held for its
    /// subscriber, and how old a subscription request may be before it is
    /// refused as stale.
    ///
    /// They are the same figure because the replay guard only bites while a
    /// record exists, so shortening this to save disk also shortens the span
    /// over which a captured subscription request can be replayed.
    #[serde(with = "humantime_serde", default = "default_retention")]
    pub retention: Duration,
}

fn default_legacy_dm() -> bool {
    true
}

fn default_publish_kinds() -> Vec<u32> {
    vec![1059, 4]
}

fn default_retention() -> Duration {
    Duration::from_secs(7 * 24 * 3600)
}

impl Default for NostrConfig {
    fn default() -> Self {
        Self {
            upstreams: Vec::new(),
            publish: false,
            allow_npubs: Vec::new(),
            legacy_dm: default_legacy_dm(),
            publish_kinds: default_publish_kinds(),
            retention: default_retention(),
        }
    }
}

impl NostrConfig {
    pub(crate) fn inbound_kinds(&self) -> Vec<u32> {
        let mut kinds = vec![1059];
        if self.legacy_dm {
            kinds.push(4);
        }
        kinds
    }

    /// Gates both verbs, not just the read side: gating only subscriptions
    /// would leave a personal bridge relaying strangers' events to public
    /// relays over its operator's own connection, which is the thing an
    /// operator who fills the list is trying to prevent.
    pub(crate) fn may_use(&self, pubkey: &[u8; 32]) -> bool {
        if self.allow_npubs.is_empty() {
            return true;
        }

        self.allow_npubs
            .iter()
            .any(|npub| match crate::npub::to_pubkey(npub) {
                Some(decoded) => decoded == *pubkey,
                None => {
                    tracing::warn!("invalid npub in allow_npubs: {}", npub);
                    false
                }
            })
    }
}

#[cfg(test)]
mod tests;
