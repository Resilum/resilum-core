use serde::{Deserialize, Serialize};

use crate::ble::links::PeerId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HostsWhileOnARouter {
    Refused,
    #[default]
    NoOneCanSay,
    Confirmed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    pub has_an_uplink: bool,
    pub p2p_and_sta_at_once: HostsWhileOnARouter,
    pub charging: bool,
    pub battery_percent: u8,
    pub neighbours_heard: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub(super) who: PeerId,
    pub(super) facts: Facts,
    pub(super) can_host_at_all: bool,
}

impl Candidate {
    #[must_use]
    pub fn heard_on_the_air(who: PeerId, facts: Facts, can_host_at_all: bool) -> Self {
        Self {
            who,
            facts,
            can_host_at_all,
        }
    }

    #[must_use]
    pub fn who(&self) -> PeerId {
        self.who
    }

    #[must_use]
    pub fn facts(&self) -> Facts {
        self.facts
    }

    #[must_use]
    pub fn can_host_at_all(&self) -> bool {
        self.can_host_at_all
    }
}

pub type HighestRankFirst = (bool, HostsWhileOnARouter, bool, u8, u8);

pub type RanksSteadyEnoughToHandOverFor = (bool, HostsWhileOnARouter, bool);

#[must_use]
pub fn rank(facts: &Facts) -> HighestRankFirst {
    (
        facts.has_an_uplink,
        facts.p2p_and_sta_at_once,
        facts.charging,
        facts.battery_percent.min(100),
        facts.neighbours_heard,
    )
}

#[must_use]
pub fn worth_a_handover(facts: &Facts) -> RanksSteadyEnoughToHandOverFor {
    (
        facts.has_an_uplink,
        facts.p2p_and_sta_at_once,
        facts.charging,
    )
}

#[must_use]
pub fn the_one_to_host(candidates: &[Candidate]) -> Option<Candidate> {
    candidates
        .iter()
        .filter(|candidate| candidate.can_host_at_all)
        .copied()
        .max_by_key(|candidate| (rank(&candidate.facts), std::cmp::Reverse(candidate.who)))
}

#[cfg(test)]
mod tests;
