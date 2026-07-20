//! Egress candidate model, eligibility filtering and selection policy.

mod candidate;
mod eligibility;
pub mod listen;
mod selector;

pub use candidate::{Candidate, CandidateRegistry};
pub use eligibility::eligible;
pub use selector::choose_best;
