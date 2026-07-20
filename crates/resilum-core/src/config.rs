use std::path::PathBuf;

use crate::spec::Specs;

/// Typed node configuration. Grows as the port progresses.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub instance_name: String,
    /// leviculum `storage_path`; state is not persisted when unset.
    pub storage_path: Option<PathBuf>,
    /// Public TCP listen address, e.g. `[::]:4242`.
    pub listen: Option<String>,
    /// Bootstrap/anchor `host:port` addresses.
    pub bootstrap: Vec<String>,
    /// Enable the local-segment AutoInterface.
    pub discover_interfaces: bool,
    /// Bridge/VPN/covert specs to run under supervision.
    pub specs: Specs,
}

impl Config {
    pub fn minimal(instance_name: impl Into<String>) -> Self {
        Self {
            instance_name: instance_name.into(),
            storage_path: None,
            listen: None,
            bootstrap: Vec::new(),
            discover_interfaces: true,
            specs: Specs::default(),
        }
    }
}
