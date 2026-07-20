//! Egress candidate model, eligibility filtering and selection policy.

mod candidate;
pub mod connect;
pub mod discover;
mod eligibility;
pub mod listen;
pub mod monitor;
pub mod probe;
mod selector;

pub use candidate::{Candidate, CandidateRegistry};
pub use eligibility::eligible;
pub use selector::choose_best;
