//! Why the router would not take a message, translated into what the caller
//! should do about it.

use leviculum_lxmf::router::RouterError;

/// What the caller should do next. A token rather than a boolean: a boolean
/// cannot tell "send it again" from "send it another way".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Retry {
    /// Send the same request again later — nothing about it was wrong.
    Resubmit,
    /// Re-queue the held message under another delivery method.
    Requeue,
    /// The request itself is malformed; repeating it verbatim cannot succeed.
    Never,
}

impl Retry {
    pub(super) fn token(self) -> &'static str {
        match self {
            Self::Resubmit => "resubmit",
            Self::Requeue => "requeue",
            Self::Never => "never",
        }
    }
}

/// A refusal as the caller reads it. One value rather than two arguments, so
/// the reason and the action cannot be separated or handed over in the wrong
/// order on the way to the JSON.
#[derive(Clone, Copy, Debug)]
pub(super) struct Refusal {
    pub(super) reason: &'static str,
    pub(super) retry: Retry,
}

pub(super) const ATTEMPTS_EXHAUSTED: Refusal = refuse("attempts_exhausted", Retry::Resubmit);

/// `Never` only where the request itself is what is wrong, since repeating an
/// identical call cannot change that.
///
/// `Requeue` where the router is still carrying the message and the obstacle
/// is the propagation path specifically, so another delivery method is the
/// remedy rather than waiting. Everything else is `Resubmit`, on the judgment
/// that telling a caller to give up on something that could still succeed is
/// the worse mistake.
pub(super) fn refusal(error: &RouterError) -> Refusal {
    match error {
        RouterError::UnsupportedMethod => refuse("unsupported_method", Retry::Never),
        RouterError::IdentityMismatch => refuse("identity_mismatch", Retry::Never),
        RouterError::NotFound => refuse("not_found", Retry::Never),
        RouterError::NoWallClock => refuse("no_wall_clock", Retry::Never),
        RouterError::Message(_) => refuse("malformed_message", Retry::Never),
        RouterError::Duplicate => refuse("duplicate", Retry::Requeue),
        RouterError::PropagationNodeUnavailable => {
            refuse("propagation_node_unavailable", Retry::Requeue)
        }
        RouterError::PropagationStampUnavailable => {
            refuse("propagation_stamp_unavailable", Retry::Requeue)
        }
        RouterError::Propagation(_) => refuse("propagation_failed", Retry::Requeue),
        RouterError::PropagationTransport(_) => refuse("propagation_transport", Retry::Requeue),
        RouterError::QueueFull => refuse("queue_full", Retry::Resubmit),
        RouterError::StaleStampRequest => refuse("stale_stamp_request", Retry::Resubmit),
        RouterError::StaleBuild => refuse("stale_build", Retry::Resubmit),
        RouterError::Node(_) => refuse("lxmf_node", Retry::Resubmit),
        RouterError::Paper(_) => refuse("paper_format", Retry::Resubmit),
        RouterError::Stamp(_) => refuse("stamp_failed", Retry::Resubmit),
        RouterError::Storage(_) => refuse("storage_failed", Retry::Resubmit),
        RouterError::CorruptSnapshot => refuse("corrupt_snapshot", Retry::Resubmit),
    }
}

const fn refuse(reason: &'static str, retry: Retry) -> Refusal {
    Refusal { reason, retry }
}

#[cfg(test)]
mod tests;
