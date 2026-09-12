//! Rendezvous protocol: peers request the local endpoint over an encrypted
//! link instead of reading it from a broadcast announce.

mod client;
mod responder;

pub(crate) const ENDPOINT_PATH: &str = "endpoint";
pub(crate) const REQUEST_TIMEOUT_MS: u64 = 15_000;

pub use client::fetch_endpoint;
pub use responder::{build_destinations, run_announcer, run_responder};

fn as_one_msgpack_value(payload: &[u8]) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec(serde_bytes::Bytes::new(payload))
}

fn from_one_msgpack_value(raw: &[u8]) -> Option<Vec<u8>> {
    rmp_serde::from_slice::<serde_bytes::ByteBuf>(raw)
        .ok()
        .map(serde_bytes::ByteBuf::into_vec)
}

#[cfg(test)]
mod tests;
