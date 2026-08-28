use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;

use crate::config::CovertDiscoveryService;
use crate::discovery::{self, covert::AddressSource};
use crate::error::Result;
use crate::node::Node;

pub(super) fn bring_up(
    node: &mut Node,
    engine: &Arc<ReticulumNode>,
    identity: &Identity,
    addresses: &[Arc<AddressSource>],
) -> Result<()> {
    let rendezvous = discovery::covert::rendezvous::build_destinations(
        engine,
        identity.clone(),
        &node.config.covert_discovery,
    )?;
    if rendezvous.is_empty() {
        return Ok(());
    }
    node.tasks
        .push(tokio::spawn(discovery::covert::rendezvous::run_announcer(
            engine.clone(),
            rendezvous.into_iter().map(|(_, hash)| hash).collect(),
            node.config.discovery_announce_interval,
            node.discovery_trigger.clone(),
        )));
    for (cfg, addresses) in node.config.covert_discovery.iter().zip(addresses) {
        node.tasks
            .push(tokio::spawn(discovery::covert::rendezvous::run_responder(
                engine.clone(),
                cfg.carrier.clone(),
                addresses.clone(),
                node.events.subscribe(),
            )));
        if let Some(listener) = start_listener_if_this_host_can(engine, cfg, identity) {
            node.covert_listeners.push(listener);
        }
    }
    Ok(())
}

fn start_listener_if_this_host_can(
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
