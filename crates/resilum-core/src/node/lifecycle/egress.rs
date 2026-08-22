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
        let ours = egress::own::OwnExits::of(&node.config.egress, |service| {
            egress::listen::dest_hash(identity.clone(), service)
        });
        for exit in ours.with_a_socket_this_node_can_dial(&ingress.services) {
            node.registry
                .upsert(&exit.service, exit.dest_hash.clone(), &exit.exit_country);
        }
        let active = Arc::new(egress::ActiveLinks::default());
        if let Some(target) = ingress.target {
            if let Some(first) = ingress.services.first() {
                node.registry.upsert(first, target.to_vec(), "*");
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
                    ours.hashes_serving(service),
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
            ours.clone(),
        )));
        node.tasks.push(tokio::spawn(egress::monitor::run(
            engine.clone(),
            router.clone(),
            node.registry.clone(),
            ingress,
            ours,
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
