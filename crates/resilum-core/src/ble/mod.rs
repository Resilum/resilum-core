#[cfg(feature = "ble")]
pub mod backend;
pub mod beacon;
pub mod direction;
pub mod election;
pub mod framing;
pub mod handshake;
pub mod link;
pub mod links;
pub mod radio;
pub mod run;
pub mod spec;

pub const ATTACHED_AS: &str = "ble";
