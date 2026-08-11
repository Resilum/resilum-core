//! LXMF messaging, end to end.
//!
//! These exercise the real path — the router runs inside the engine's tick, the
//! announce carries the delivery destination, the message rides an actual
//! interface, and the queue is written to disk. A unit test over the JSON
//! mapping cannot tell whether any of that is wired up.

mod common;
mod delivery;
mod durability;
