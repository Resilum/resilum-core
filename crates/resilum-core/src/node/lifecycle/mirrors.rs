use std::sync::Arc;

use leviculum_std::api::{Identity, Node as LevNode};

use crate::mirrors::{self, Registry};
use crate::node::Node;

pub(super) fn bring_up(node: &mut Node, engine: &Arc<LevNode>, identity: &Identity) {
    let registry_path = node
        .config
        .storage_path
        .as_ref()
        .map(|p| p.join("mirror_registry.json"));
    let registry = Arc::new(Registry::new(registry_path));
    node.mirror_registry = Some(registry.clone());

    node.tasks.push(tokio::spawn(mirrors::consume(
        registry,
        node.events.subscribe(),
    )));

    if !node.config.advertised_mirrors.is_empty()
        && let Some(rngit_file) = node.config.rngit_destination_file.clone()
    {
        node.tasks.push(tokio::spawn(mirrors::produce(
            engine.clone(),
            identity.clone(),
            node.config.discovery_announce_interval,
            node.config.advertised_mirrors.clone(),
            rngit_file,
        )));
    }
}
