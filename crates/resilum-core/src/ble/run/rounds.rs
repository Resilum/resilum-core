use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use tokio::task::JoinSet;

use crate::ble::beacon::Beacon;
use crate::ble::election::{
    Election, Field, HostingTheGroup, Verdict, WhatThePlatformKnows, exchange,
};
use crate::ble::links::PeerId;
use crate::ble::radio::Radio;
use crate::ble::spec;
use crate::link::LinkRouter;

const CONSIDER_EVERY: Duration = Duration::from_secs(5);
const ASK_A_CANDIDATE_EVERY_MS: u64 = 10_000;
const HEAR_A_NEIGHBOUR_OUT_MS: u64 = 30_000;
const LOOK_AROUND_BEFORE_JUDGING_MS: u64 = 15_000;

pub struct Deciding {
    pub engine: Arc<ReticulumNode>,
    pub identity: Identity,
    pub router: Arc<LinkRouter>,
    pub radio: Arc<dyn Radio>,
    pub field: Field,
    pub known: WhatThePlatformKnows,
    pub hosting: HostingTheGroup,
    pub us: PeerId,
    pub beacon: Beacon,
}

pub async fn keep_deciding(mut deciding: Deciding, since: Instant) {
    let now_ms = move || u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX);
    let mut election = Election::watching(deciding.us);
    let mut asked_at: HashMap<PeerId, u64> = HashMap::new();
    let mut asking = JoinSet::new();
    loop {
        let now = now_ms();
        while asking.try_join_next().is_some() {}
        ask_whoever_has_not_answered_lately(&deciding, &mut asked_at, &mut asking, now);
        decide_once_the_field_has_spoken(&mut deciding, &mut election, now);
        tokio::select! {
            () = tokio::time::sleep(CONSIDER_EVERY) => {}
            () = deciding.field.until_someone_new() => {}
        }
    }
}

fn the_field_is_not_known_yet(field: &Field, now_ms: u64) -> bool {
    now_ms < LOOK_AROUND_BEFORE_JUDGING_MS
        || field.someone_met_is_still_worth_hearing_out(now_ms, HEAR_A_NEIGHBOUR_OUT_MS)
}

fn decide_once_the_field_has_spoken(deciding: &mut Deciding, election: &mut Election, now_ms: u64) {
    if the_field_is_not_known_yet(&deciding.field, now_ms) {
        return;
    }
    let ours = exchange::what_we_tell(&deciding.known, &deciding.field);
    let standing = deciding
        .field
        .standing_with_us(deciding.us, ours.facts, ours.can_host_at_all);
    match election.consider(&standing, now_ms) {
        Verdict::CarryOn => {}
        Verdict::RaiseTheGroup => deciding.hosting.raise(),
        Verdict::StandDown => deciding.hosting.stand_down(),
    }
    say_on_the_air_what_we_became(deciding);
}

fn ask_whoever_has_not_answered_lately(
    deciding: &Deciding,
    asked_at: &mut HashMap<PeerId, u64>,
    asking: &mut JoinSet<()>,
    now_ms: u64,
) {
    let ours = exchange::what_we_tell(&deciding.known, &deciding.field);
    let standing = deciding.field.whom_we_hear();
    for peer in &standing {
        let rested = asked_at
            .get(peer)
            .is_some_and(|at| now_ms.saturating_sub(*at) < ASK_A_CANDIDATE_EVERY_MS);
        if rested {
            continue;
        }
        asked_at.insert(*peer, now_ms);
        asking.spawn(exchange::ask(
            deciding.engine.clone(),
            deciding.router.clone(),
            deciding.field.clone(),
            deciding.identity.clone(),
            *peer,
            ours,
        ));
    }
    asked_at.retain(|peer, _| standing.contains(peer));
}

fn say_on_the_air_what_we_became(deciding: &mut Deciding) {
    let now = Beacon {
        can_host: deciding.known.can_host_at_all(),
        group_is_up: deciding.hosting.is_up(),
        ..deciding.beacon
    };
    if now == deciding.beacon {
        return;
    }
    deciding.beacon = now;
    if let Err(error) = deciding.radio.advertise(&now.name(), spec::SERVICE) {
        tracing::warn!(?error, "ble re-advertise refused");
    }
}

#[cfg(test)]
mod tests;
