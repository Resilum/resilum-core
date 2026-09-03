mod a_radio_here;

use std::sync::Arc;
use std::time::{Duration, Instant};

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;

use crate::ble::beacon::Beacon;
use crate::ble::election::{Field, exchange};
use crate::ble::links::PeerId;
use crate::ble::radio::Radio;
use crate::ble::run::{self, Deciding, Ours};
use crate::ble::spec;
use crate::link::{Inbox, LinkRouter};
use crate::node::Node;

const ANNOUNCE_EVERY: Duration = Duration::from_secs(600);

pub(crate) struct Wiring<'a> {
    pub engine: &'a Arc<ReticulumNode>,
    pub identity: &'a Identity,
    pub router: &'a Arc<LinkRouter>,
    pub inbox: &'a Arc<Inbox>,
}

pub(super) fn bring_up(node: &mut Node, wiring: &Wiring) {
    if node.config.ble.is_none() {
        return;
    }
    let mut ours = [0u8; spec::IDENTITY_LEN];
    ours.copy_from_slice(&wiring.identity.hash()[..spec::IDENTITY_LEN]);
    speak_over(node, wiring, ours, a_radio_here::opened());
}

pub(crate) fn speak_over<R>(node: &mut Node, wiring: &Wiring, ours: PeerId, opening: R)
where
    R: std::future::Future<Output = Option<Arc<dyn Radio>>> + Send + 'static,
{
    let field = node.ble_field.clone();
    answer_other_candidates(node, wiring, &field);
    let engine = wiring.engine.clone();
    let identity = wiring.identity.clone();
    let router = wiring.router.clone();
    let known = node.ble_facts.clone();
    let hosting = node.ble_hosting.clone();
    let attachments = node.attachments.clone();
    let origins = node.origin_registry.clone();
    let nursery = node.nursery.clone();
    node.tasks.push(node.runtime.handle().spawn(async move {
        let Some(radio) = opening.await else {
            return;
        };
        let Some(events) = radio.events_taken_once() else {
            return;
        };
        let beacon = Beacon::fresh(known.can_host_at_all(), hosting.is_up());
        radio.serve_identity(ours);
        if let Err(e) = radio.advertise(&beacon.name(), spec::SERVICE) {
            tracing::warn!(error = ?e, "ble advertise refused");
        }
        if let Err(e) = radio.scan(spec::SERVICE) {
            tracing::warn!(error = ?e, "ble scan refused");
        }
        let since = Instant::now();
        let group_falls_with_the_radio = hosting.clone();
        tokio::select! {
            () = run::run(
                Ours {
                    engine: engine.clone(),
                    radio: radio.clone(),
                    identity: ours,
                    beacon,
                    attachments,
                    origins,
                    field: field.clone(),
                },
                events,
                since,
            ) => {}
            () = run::keep_deciding(
                Deciding {
                    engine,
                    identity,
                    router,
                    radio,
                    field,
                    known,
                    hosting,
                    us: ours,
                    beacon,
                    nursery,
                },
                since,
            ) => {}
        }
        group_falls_with_the_radio.stand_down();
    }));
}

fn answer_other_candidates(node: &mut Node, wiring: &Wiring, field: &Field) {
    let destination = exchange::destination(wiring.identity.clone());
    let ours = *destination.hash();
    wiring.engine.register_destination(destination);
    let asked_of_us = wiring.inbox.claim(*ours.as_bytes());
    node.tasks
        .push(node.runtime.handle().spawn(exchange::answer(
            wiring.engine.clone(),
            asked_of_us,
            node.ble_facts.clone(),
            field.clone(),
            node.nursery.clone(),
        )));
    node.tasks
        .push(node.runtime.handle().spawn(crate::announce_ours::every(
            wiring.engine.clone(),
            ours,
            ANNOUNCE_EVERY,
        )));
}
