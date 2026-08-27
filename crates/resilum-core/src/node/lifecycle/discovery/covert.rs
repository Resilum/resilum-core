use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;

use crate::config::CovertDiscoveryService;
use crate::discovery;

pub(super) fn start_listener_if_this_host_can(
    engine: &Arc<ReticulumNode>,
    cfg: &CovertDiscoveryService,
    identity: &Identity,
) -> Option<ByteChannelHandle> {
    let name = format!("CovertListen[{}]", cfg.carrier);
    match discovery::covert::listen(engine, &name, &cfg.carrier, identity.clone(), cfg.mtu) {
        Ok(handle) => {
            tracing::info!(%name, "covert listener attached");
            Some(handle)
        }
        Err(e) => {
            tracing::warn!(%name, error = %e, "covert listener unavailable");
            None
        }
    }
}
