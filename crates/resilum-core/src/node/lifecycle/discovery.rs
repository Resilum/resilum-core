use std::sync::Arc;

use leviculum_std::api::{Identity, Node as LevNode};

#[cfg(feature = "arti")]
use crate::error::Error;
use crate::error::Result;
use crate::node::Node;
use crate::{announce_cap, announce_trigger, discovery};

pub(super) fn bring_up(node: &mut Node, engine: &Arc<LevNode>, identity: &Identity) -> Result<()> {
    if node.config.discovery.is_empty() && node.config.covert_discovery.is_empty() {
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
    let embedded_tor = if wants_embedded_arti(&node.config.discovery) {
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
    let (discovery, ygg_discovery) = discovery::build_from_services(discovery::BuildParams {
        tcp: &discovery_cfg,
        covert: &node.config.covert_discovery,
        covert_addresses: &covert_addresses,
        engine: engine.clone(),
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
    let plugins = Arc::new(discovery);
    let bus = node.events.subscribe();
    node.tasks
        .push(tokio::spawn(discovery::run_consume(plugins.clone(), bus)));

    let mut destinations = discovery::build_destinations(engine, identity.clone(), &discovery_cfg)?;
    destinations.extend(discovery::covert::rendezvous::build_destinations(
        engine,
        identity.clone(),
        &node.config.covert_discovery,
    )?);
    for (cfg, addresses) in node.config.covert_discovery.iter().zip(&covert_addresses) {
        node.tasks
            .push(tokio::spawn(discovery::covert::rendezvous::run_responder(
                engine.clone(),
                cfg.carrier.clone(),
                addresses.clone(),
                node.events.subscribe(),
            )));
    }
    node.tasks.push(tokio::spawn(discovery::run_produce(
        engine.clone(),
        plugins,
        destinations,
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

#[cfg(feature = "arti")]
fn wants_embedded_arti(services: &[crate::config::DiscoveryService]) -> bool {
    use crate::config::SocksProxy;
    services
        .iter()
        .any(|s| matches!(s.socks_proxy, Some(SocksProxy::EmbeddedArti)))
}

fn resolve_discovery(
    services: &[crate::config::DiscoveryService],
    #[cfg(feature = "arti")] arti_port: Option<u16>,
) -> Vec<crate::config::DiscoveryService> {
    services
        .iter()
        .map(|s| {
            let out = s.clone();
            #[cfg(feature = "arti")]
            let out = {
                use crate::config::SocksProxy;
                let mut out = out;
                if let Some(SocksProxy::EmbeddedArti) = out.socks_proxy
                    && let Some(port) = arti_port
                {
                    out.socks_proxy = Some(SocksProxy::External("127.0.0.1".into(), port));
                }
                out
            };
            out
        })
        .collect()
}
