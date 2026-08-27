mod covert;
mod resolve;

use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;

use crate::config::Config;
#[cfg(feature = "arti")]
use crate::error::Error;
use crate::error::Result;
use crate::node::Node;
use crate::{announce_cap, announce_trigger, discovery};
use resolve::resolve_discovery;

fn wants_discovery(config: &Config) -> bool {
    !config.discovery.is_empty()
        || !config.covert_discovery.is_empty()
        || (cfg!(feature = "iroh") && config.iroh.is_some())
        || !config.advertised_services.is_empty()
}

pub(super) fn bring_up(
    node: &mut Node,
    engine: &Arc<ReticulumNode>,
    identity: &Identity,
) -> Result<()> {
    if !wants_discovery(&node.config) {
        return Ok(());
    }

    let storage_root = node.config.storage_path.as_deref();
    let cap_controller = announce_cap::CapController::new(engine.clone());
    node.tasks
        .push(tokio::spawn(announce_cap::run(cap_controller.clone())));
    node.tasks.push(tokio::spawn(announce_trigger::run(
        engine.clone(),
        node.discovery_trigger.clone(),
    )));

    #[cfg(feature = "arti")]
    let embedded_tor = if resolve::wants_embedded_arti(&node.config.discovery) {
        Some(
            node.runtime
                .block_on(crate::tor::EmbeddedTor::spawn(storage_root))
                .map_err(|e| Error::Engine(format!("arti bootstrap: {e}")))?,
        )
    } else {
        None
    };
    let discovery_cfg = resolve_discovery(
        &node.config.discovery,
        #[cfg(feature = "arti")]
        embedded_tor.as_ref().map(|t| t.port()),
    );

    let covert_addresses = discovery::build_covert_addresses(&node.config.covert_discovery);
    let (mut discovery, ygg_discovery) = discovery::build_from_services(discovery::BuildParams {
        tcp: &discovery_cfg,
        covert: &node.config.covert_discovery,
        covert_addresses: &covert_addresses,
        engine: engine.clone(),
        coordinates: node.coordinates.clone(),
        attachments: node.attachments.clone(),
        trigger: node.discovery_trigger.clone(),
        storage_root,
        cap_controller,
        events: node.events.clone(),
        origin_registry: node.origin_registry.clone(),
    });
    #[cfg(all(unix, feature = "ygg"))]
    {
        node.ygg_discovery = ygg_discovery;
    }
    #[cfg(not(all(unix, feature = "ygg")))]
    let _ = ygg_discovery;
    for (service, directory) in &node.directories {
        discovery.register(*service, directory.clone());
    }
    #[cfg(feature = "iroh")]
    if node.config.iroh.is_some() {
        let plugin = Arc::new(crate::iroh::IrohDiscovery::default());
        discovery.register(discovery::Service::IROH, plugin.clone());
        node.iroh_discovery = Some(plugin);
    }
    let plugins = Arc::new(discovery);
    let bus = node.events.subscribe();
    node.tasks
        .push(tokio::spawn(discovery::run_consume(plugins.clone(), bus)));

    let destination = discovery::build_destination(engine, identity.clone())?;
    discovery::covert::rendezvous::build_destinations(
        engine,
        identity.clone(),
        &node.config.covert_discovery,
    )?;
    for (cfg, addresses) in node.config.covert_discovery.iter().zip(&covert_addresses) {
        node.tasks
            .push(tokio::spawn(discovery::covert::rendezvous::run_responder(
                engine.clone(),
                cfg.carrier.clone(),
                addresses.clone(),
                node.events.subscribe(),
            )));
        if let Some(listener) = covert::start_listener_if_this_host_can(engine, cfg, identity) {
            node.covert_listeners.push(listener);
        }
    }
    node.tasks.push(tokio::spawn(discovery::run_produce(
        engine.clone(),
        plugins,
        destination,
        node.config.discovery_announce_interval,
        node.discovery_trigger.clone(),
    )));
    if let Some(root) = node.config.storage_path.clone() {
        let services = discovery_cfg.iter().map(|s| s.service.clone()).collect();
        node.tasks
            .push(tokio::spawn(discovery::run_prune_loop(root, services)));
    }

    #[cfg(feature = "arti")]
    {
        node.embedded_tor = embedded_tor;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::wants_discovery;
    use crate::config::Config;

    #[test]
    fn a_bare_config_wants_no_discovery() {
        assert!(!wants_discovery(&Config::minimal("t")));
    }

    #[test]
    fn a_node_that_will_advertise_wants_discovery_on_its_own() {
        let cfg = Config {
            advertised_services: vec![crate::discovery::Service::NOSTR_RELAY],
            ..Config::minimal("t")
        };
        assert!(wants_discovery(&cfg));
    }
}
