use crate::error::{Error, Result};
use crate::node::Node;

impl Node {
    pub fn iroh_attach(&self) -> Result<crate::iroh::IrohHandle> {
        let engine = self.engine.clone().ok_or(Error::NotRunning)?;
        let cfg = self
            .config
            .iroh
            .clone()
            .ok_or_else(|| Error::Iroh("iroh not configured".into()))?;
        let dir = self.config.storage_path.clone().unwrap_or_else(|| {
            std::env::temp_dir().join(format!("resilum-{}", self.config.instance_name))
        });
        let discovery = self.iroh_discovery.clone();
        let protect = self.protect.clone();
        let handle = self
            .runtime
            .block_on(crate::iroh::attach(crate::iroh::Attaching {
                engine,
                where_the_secret_lives: &dir,
                cfg: &cfg,
                discovery,
                protect,
                attachments: self.attachments.clone(),
                origin: self.origin_registry.clone(),
                arriving: self.nursery.clone(),
            }))
            .map_err(Error::Iroh)?;
        self.discovery_trigger.notify_waiters();
        Ok(handle)
    }
}
