//! An event from the mesh, offered to every relay, then answered for.

use std::collections::HashMap;

pub(in crate::bridge) use self::admit::Source;
pub(in crate::bridge) use self::publishing::Publishing;
use self::round::Publication;
pub(in crate::bridge) use self::round::Verdict;

mod accept;
mod admit;
mod pending;
mod publishing;
mod refusal;
mod round;
mod settle;

/// Events offered to the relays and not yet answered for, one per event id.
/// Owned by the loop rather than shared: nothing outside it counts a verdict.
#[derive(Default)]
pub(super) struct Pending {
    awaiting: HashMap<String, Publication>,
}

#[cfg(test)]
mod tests;
