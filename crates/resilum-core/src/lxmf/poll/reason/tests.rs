use std::collections::HashSet;

use leviculum_lxmf::router::RouterError;
use leviculum_lxmf::storage::StorageError;
use leviculum_lxmf::{
    LxmfNodeError, MessageError, PaperError, PropagationError, PropagationTransportError,
    StampError,
};

use super::{Retry, refusal};

/// One instance per variant, hand-written because `RouterError` carries data.
/// [`slot_of`] is what keeps the list honest.
fn all_variants() -> Vec<RouterError> {
    vec![
        RouterError::QueueFull,
        RouterError::Duplicate,
        RouterError::NotFound,
        RouterError::IdentityMismatch,
        RouterError::UnsupportedMethod,
        RouterError::PropagationNodeUnavailable,
        RouterError::PropagationStampUnavailable,
        RouterError::StaleStampRequest,
        RouterError::StaleBuild,
        RouterError::NoWallClock,
        RouterError::Node(LxmfNodeError::UnknownPeer),
        RouterError::Message(MessageError::TooShort),
        RouterError::Propagation(PropagationError::Truncated),
        RouterError::PropagationTransport(PropagationTransportError::UnknownNode),
        RouterError::Paper(PaperError::TooShort),
        RouterError::Stamp(StampError::InvalidCost),
        RouterError::Storage(StorageError::Full),
        RouterError::CorruptSnapshot,
    ]
}

const VARIANT_COUNT: usize = 18;

/// A slot per variant, assigned by an exhaustive match. A new upstream variant
/// has to be given one here, and `VARIANT_COUNT` raised for it, before
/// `the_variant_list_names_every_router_error` can pass — which it only does
/// once `all_variants` carries an instance of it too.
fn slot_of(error: &RouterError) -> usize {
    match error {
        RouterError::QueueFull => 0,
        RouterError::Duplicate => 1,
        RouterError::NotFound => 2,
        RouterError::IdentityMismatch => 3,
        RouterError::UnsupportedMethod => 4,
        RouterError::PropagationNodeUnavailable => 5,
        RouterError::PropagationStampUnavailable => 6,
        RouterError::StaleStampRequest => 7,
        RouterError::StaleBuild => 8,
        RouterError::NoWallClock => 9,
        RouterError::Node(_) => 10,
        RouterError::Message(_) => 11,
        RouterError::Propagation(_) => 12,
        RouterError::PropagationTransport(_) => 13,
        RouterError::Paper(_) => 14,
        RouterError::Stamp(_) => 15,
        RouterError::Storage(_) => 16,
        RouterError::CorruptSnapshot => 17,
    }
}

const NEVER: [&str; 5] = [
    "unsupported_method",
    "identity_mismatch",
    "not_found",
    "no_wall_clock",
    "malformed_message",
];

const REQUEUE: [&str; 5] = [
    "duplicate",
    "propagation_node_unavailable",
    "propagation_stamp_unavailable",
    "propagation_failed",
    "propagation_transport",
];

#[test]
fn the_variant_list_names_every_router_error() {
    let mut seen = [false; VARIANT_COUNT];
    for error in all_variants() {
        seen[slot_of(&error)] = true;
    }
    let missing: Vec<usize> = (0..VARIANT_COUNT).filter(|slot| !seen[*slot]).collect();
    assert!(missing.is_empty(), "no instance for slots {missing:?}");
}

#[test]
fn every_variant_gets_a_reason_no_other_variant_uses() {
    let mut seen = HashSet::new();
    for error in all_variants() {
        let reason = refusal(&error).reason;
        assert!(seen.insert(reason), "{error:?} reuses reason {reason:?}");
    }
}

#[test]
fn the_retry_token_matches_what_the_caller_can_actually_do() {
    for error in all_variants() {
        let refusal = refusal(&error);
        let expected = if NEVER.contains(&refusal.reason) {
            Retry::Never
        } else if REQUEUE.contains(&refusal.reason) {
            Retry::Requeue
        } else {
            Retry::Resubmit
        };
        assert_eq!(refusal.retry, expected, "{}", refusal.reason);
    }
}
