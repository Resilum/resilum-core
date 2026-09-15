//! The only place the parts meet.

pub use self::start::{BridgeHandle, StartError, spawn};

mod deliver;
mod dispatch;
mod from_mesh;
mod maintain;
mod publish;
mod recent;
mod relay;
mod retry;
mod run;
mod schema;
mod start;
mod state;
mod subscribe;
mod tie;
mod to_mesh;
