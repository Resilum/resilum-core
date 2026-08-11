//! Per-entry YAML types + their conversions to core config values.

mod discovery;
mod lxmf;
mod services;

pub(super) use discovery::DiscoveryFile;
pub(super) use lxmf::LxmfFile;
pub(super) use services::{EgressFile, I2pFile, IngressFile};
