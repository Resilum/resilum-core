/// Node configuration — the typed equivalent of what the container entrypoint
/// renders today. Fields are placeholders, filled in as the port progresses.
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Human-readable instance name.
    pub instance_name: String,
    /// TCP listen address for the public interface, if any (e.g. `[::]:4242`).
    pub listen: Option<String>,
    /// Bootstrap / anchor addresses used to seed connectivity.
    pub bootstrap: Vec<String>,
    /// Whether interface discovery is enabled.
    pub discover_interfaces: bool,
}

impl Config {
    /// A minimal config for a bare node / tests.
    pub fn minimal(instance_name: impl Into<String>) -> Self {
        Self {
            instance_name: instance_name.into(),
            listen: None,
            bootstrap: Vec::new(),
            discover_interfaces: true,
        }
    }
}
