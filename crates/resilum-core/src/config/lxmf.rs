use std::time::Duration;

use leviculum_std::DestinationHash;

/// LXMF messaging. Presence of the section is what enables it.
#[derive(Clone, Debug)]
pub struct LxmfConfig {
    pub display_name: Option<String>,
    pub announce_interval: Duration,
    /// Propagation node that holds messages while we are offline. `None` uses
    /// the nearest node that announced one; set it to pin a node you run.
    pub propagation_node: Option<DestinationHash>,
}

impl Default for LxmfConfig {
    fn default() -> Self {
        Self {
            display_name: None,
            announce_interval: super::default_announce_interval(),
            propagation_node: None,
        }
    }
}
