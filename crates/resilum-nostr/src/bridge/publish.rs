//! An event from the mesh, offered to every relay, then answered for.

mod accept;
mod admit;
mod pending;
mod publishing;
mod refusal;
mod round;
mod settle;

use std::collections::HashMap;

pub(in crate::bridge) use admit::Source;
pub(in crate::bridge) use publishing::Publishing;
use round::Publication;
pub(in crate::bridge) use round::Verdict;

/// Events offered to the relays and not yet answered for, one per event id.
/// Owned by the loop rather than shared: nothing outside it counts a verdict.
#[derive(Default)]
pub(super) struct Pending {
    awaiting: HashMap<String, Publication>,
}

#[cfg(test)]
mod tests;
