//! Egress candidate model, eligibility filtering and selection policy.

pub mod active;
mod candidate;
mod choose;
pub mod discover;
mod eligibility;
pub mod ingress;
pub mod listen;
pub mod monitor;
pub mod own;
pub mod probe;
mod relay;
mod selector;
mod socks5;
pub mod vpn;

pub use self::active::ActiveLinks;
pub use self::candidate::{Candidate, CandidateRegistry};
pub use self::choose::best_available;
pub use self::eligibility::{allowed, eligible};
pub use self::selector::choose_best;
