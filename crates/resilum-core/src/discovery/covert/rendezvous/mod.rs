//! Rendezvous protocol: peers request the local endpoint over an encrypted
//! link instead of reading it from a broadcast announce.

mod client;
mod responder;

pub(crate) const ENDPOINT_PATH: &str = "endpoint";
pub(crate) const REQUEST_TIMEOUT_MS: u64 = 15_000;

pub use client::fetch_endpoint;
pub use responder::{build_destinations, run_responder};
