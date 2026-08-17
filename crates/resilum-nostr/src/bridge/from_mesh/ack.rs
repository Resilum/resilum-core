//! Telling a sender what became of the event it published.
//!
//! A delivery tick would be false with no relay behind it, so this is the
//! only thing that distinguishes "sent" from "nobody took it".

use serde_json::{Value, json};

/// How many relays took the event, and why none did when that happened.
pub(in crate::bridge) struct Verdicts {
    pub(in crate::bridge) accepted: u32,
    pub(in crate::bridge) rejected: u32,
    pub(in crate::bridge) reason: String,
}

/// `reason` only matters when nothing was accepted: with at least one relay
/// taking the event, the sender has no need to know why others refused it.
pub(in crate::bridge) fn ack_json(event_id: &str, verdicts: &Verdicts) -> Value {
    let reason = if verdicts.accepted == 0 {
        verdicts.reason.as_str()
    } else {
        ""
    };
    json!({
        "event_id": event_id,
        "accepted": verdicts.accepted,
        "rejected": verdicts.rejected,
        "reason": reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_acknowledgement_carries_a_reason_only_when_nothing_was_taken() {
        let none = ack_json(
            "e0a4",
            &Verdicts {
                accepted: 0,
                rejected: 2,
                reason: "blocked".into(),
            },
        );
        assert_eq!(none["reason"], "blocked");

        let some = ack_json(
            "e0a4",
            &Verdicts {
                accepted: 1,
                rejected: 1,
                reason: "blocked".into(),
            },
        );
        assert_eq!(some["reason"], "");
        assert_eq!(some["accepted"], 1);
    }
}
