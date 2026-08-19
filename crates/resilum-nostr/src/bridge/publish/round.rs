//! One event's round with the relays.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::bridge::tie::Tie;

/// How long a sender waits on relays that will not answer. Swept from the
/// bridge loop's poll tick, so it is only as sharp as `run::POLL_EVERY`.
pub(super) const DEADLINE: Duration = Duration::from_secs(5);

/// A relay's answer to one offered event.
pub(in crate::bridge) enum Verdict {
    Accepted,
    Rejected(String),
}

impl Verdict {
    pub(in crate::bridge) fn of_ok(accepted: bool, message: String) -> Self {
        if accepted || message.starts_with("duplicate:") {
            return Self::Accepted;
        }
        Self::Rejected(message)
    }
}

/// An event offered to the relays and not yet answered for.
///
/// The round belongs to the event, not to whoever carried it here: a relay
/// judges the bytes, so every mesh peer that forwards the same event shares
/// one tally and one deadline, and they differ only in where the answer goes.
pub(super) struct Publication {
    pub(super) tie: Tie,
    pub(super) event_json: Arc<str>,
    pub(super) expected: usize,
    pub(super) accepted: u32,
    pub(super) rejected: u32,
    pub(super) reason: String,
    pub(super) deadline: Instant,
    /// Mesh addresses owed an acknowledgement, in the order they asked.
    /// Empty for a queued event offered again: its sender was answered when
    /// it first arrived.
    pub(super) reply_to: Vec<[u8; 16]>,
}

impl Publication {
    pub(super) fn new(
        tie: Tie,
        event_json: Arc<str>,
        reply_to: Vec<[u8; 16]>,
        expected: usize,
        now: Instant,
    ) -> Self {
        Self {
            tie,
            event_json,
            expected,
            accepted: 0,
            rejected: 0,
            // Stands until a relay gives a better one, so an event that went
            // nowhere says why rather than reporting a bare zero.
            reason: if expected == 0 {
                "no relay is connected"
            } else {
                "no relay answered in time"
            }
            .to_owned(),
            deadline: now + DEADLINE,
            reply_to,
        }
    }

    /// A second peer forwarding an event already in flight is owed the same
    /// answer as the first, at its own address.
    pub(super) fn owed_to(&mut self, address: [u8; 16]) {
        if !self.reply_to.contains(&address) {
            self.reply_to.push(address);
        }
    }

    pub(super) fn owed_to_each(&mut self, addresses: &[[u8; 16]]) {
        for address in addresses {
            self.owed_to(*address);
        }
    }

    pub(super) fn counted(&mut self, verdict: Verdict) {
        match verdict {
            Verdict::Accepted => self.accepted += 1,
            Verdict::Rejected(message) => {
                self.rejected += 1;
                if self.rejected == 1 {
                    self.reason = message;
                }
            }
        }
    }

    #[must_use]
    pub(super) fn answered(&self) -> bool {
        (self.accepted + self.rejected) as usize >= self.expected
    }
}
