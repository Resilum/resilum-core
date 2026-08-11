use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc;

use crate::egress;
use crate::link;
use crate::node::Node;

pub(super) fn bring_up(
    node: &mut Node,
    engine: &Arc<ReticulumNode>,
    identity: &Identity,
    router: &Arc<link::LinkRouter>,
    inbound_rx: mpsc::UnboundedReceiver<link::Inbound>,
) {
    if let Some(ingress) = node.config.ingress.clone() {
        let mut skip: HashMap<String, HashSet<Vec<u8>>> = HashMap::new();
        for own in &node.config.egress {
            let hash = egress::listen::dest_hash(identity.clone(), &own.service);
            skip.entry(own.service.clone()).or_default().insert(hash);
        }
        let active = Arc::new(egress::ActiveLinks::default());
        if let Some(target) = ingress.target {
            if let Some(first) = ingress.services.first() {
                node.registry
                    .upsert(first, target.to_vec(), "*", Vec::new());
            }
        } else {
            for service in &ingress.services {
                let bus = node.events.subscribe();
                node.tasks.push(tokio::spawn(egress::discover::run(
                    engine.clone(),
                    node.registry.clone(),
                    active.clone(),
                    node.event_queue.clone(),
                    service.clone(),
                    skip.get(service).cloned().unwrap_or_default(),
                    bus,
                )));
            }
        }
        node.tasks.push(tokio::spawn(egress::ingress::run(
            engine.clone(),
            router.clone(),
            node.registry.clone(),
            active,
            node.socks_port.clone(),
            ingress.clone(),
            skip.clone(),
        )));
        node.tasks.push(tokio::spawn(egress::monitor::run(
            engine.clone(),
            router.clone(),
            node.registry.clone(),
            ingress,
            skip,
        )));
    }

    if !node.config.egress.is_empty() {
        node.tasks.push(tokio::spawn(egress::listen::run(
            engine.clone(),
            identity.clone(),
            node.config.egress.clone(),
            inbound_rx,
        )));
    }
}
