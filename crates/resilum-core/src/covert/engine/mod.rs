//! Covert session engine: carrier-agnostic client/server on top of the
//! HMAC-authenticated datagram + sliding-window ARQ + adaptive-poll primitives.

pub mod client;
pub mod datagram;
pub mod framing;
pub mod keyx;
pub mod poll;
pub mod server;
pub mod session;
