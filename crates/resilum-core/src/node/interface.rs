//! Runtime interface attach/detach.

use super::Node;
use crate::error::{Error, Result};

impl Node {
    /// Attach an interface at runtime from a leviculum interface-config JSON,
    /// returning the assigned interface ids (several for a fan-out type such as
    /// RNodeMulti or an I2P interface with peers).
    pub fn add_interface(&self, config_json: &str) -> Result<Vec<u64>> {
        let engine = self.engine.as_ref().ok_or(Error::NotRunning)?;
        let config: leviculum_std::api::InterfaceConfig = serde_json::from_str(config_json)
            .map_err(|e| Error::Config(format!("interface config: {e}")))?;
        let ids = engine
            .spawn_interface(config)
            .map_err(|e| Error::Engine(e.to_string()))?;
        Ok(ids.into_iter().map(|id| id.0 as u64).collect())
    }

    /// Detach an interface by id. Idempotent: an unknown id is a no-op.
    pub fn remove_interface(&self, id: u64) -> Result<()> {
        let engine = self.engine.as_ref().ok_or(Error::NotRunning)?;
        engine
            .remove_interface(leviculum_std::InterfaceId(id as usize))
            .map_err(|e| Error::Engine(e.to_string()))
    }
}
