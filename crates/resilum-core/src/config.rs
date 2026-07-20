use std::path::PathBuf;

/// Node configuration — the typed equivalent of what the container entrypoint
/// renders today (`network config` `[interfaces]` + bridge specs). Grows as
/// the port progresses; this first slice covers node-level interfaces.
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Human-readable instance name.
    pub instance_name: String,
    /// Persistent state/identity directory (leviculum `storage_path`). When
    /// unset, state is not persisted across restarts.
    pub storage_path: Option<PathBuf>,
    /// TCP listen address for the public interface, if any (e.g. `[::]:4242`).
    pub listen: Option<String>,
    /// Bootstrap / anchor `host:port` addresses used to seed connectivity.
    pub bootstrap: Vec<String>,
    /// Whether the local-segment `AutoInterface` is enabled.
    pub discover_interfaces: bool,
}

impl Config {
    /// A minimal config for a bare node / tests.
    pub fn minimal(instance_name: impl Into<String>) -> Self {
        Self {
            instance_name: instance_name.into(),
            storage_path: None,
            listen: None,
            bootstrap: Vec::new(),
            discover_interfaces: true,
        }
    }
}
