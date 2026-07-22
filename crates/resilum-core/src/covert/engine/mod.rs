//! Covert session engine: carrier-agnostic client/server on top of the
//! HMAC-authenticated datagram + sliding-window ARQ + adaptive-poll primitives.

pub mod datagram;
pub mod framing;
pub mod poll;
