pub mod exchange;
mod field;
mod hosting;
mod read_here;
mod reported;
mod score;

pub use field::Field;
pub use hosting::HostingTheGroup;
pub use read_here::what_this_host_can_answer;
pub use reported::WhatThePlatformKnows;
pub use score::{Candidate, Facts, HostsWhileOnARouter, rank, the_one_to_host, worth_a_handover};

use crate::ble::links::PeerId;

const CLAIM_DELAY_MS: u64 = 3_000;
const HOLD_DOWN_MS: u64 = 120_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Watching,
    Claiming { since_ms: u64 },
    Hosting { since_ms: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    CarryOn,
    RaiseTheGroup,
    StandDown,
}

pub struct Election {
    standing: Standing,
    us: PeerId,
}

impl Election {
    #[must_use]
    pub fn watching(us: PeerId) -> Self {
        Self {
            standing: Standing::Watching,
            us,
        }
    }

    #[must_use]
    pub fn standing(&self) -> Standing {
        self.standing
    }

    pub fn consider(&mut self, field: &[Candidate], now_ms: u64) -> Verdict {
        let Some(winner) = the_one_to_host(field) else {
            return self.step_down_if_hosting(now_ms);
        };
        if winner.who == self.us {
            return self.win(now_ms);
        }
        if self.beaten_by(&winner, field) {
            return self.step_down_if_hosting(now_ms);
        }
        Verdict::CarryOn
    }

    fn win(&mut self, now_ms: u64) -> Verdict {
        match self.standing {
            Standing::Hosting { .. } => Verdict::CarryOn,
            Standing::Watching => {
                self.standing = Standing::Claiming { since_ms: now_ms };
                Verdict::CarryOn
            }
            Standing::Claiming { since_ms } if now_ms.saturating_sub(since_ms) < CLAIM_DELAY_MS => {
                Verdict::CarryOn
            }
            Standing::Claiming { .. } => {
                self.standing = Standing::Hosting { since_ms: now_ms };
                Verdict::RaiseTheGroup
            }
        }
    }

    fn beaten_by(&self, winner: &Candidate, field: &[Candidate]) -> bool {
        let Standing::Hosting { .. } = self.standing else {
            return true;
        };
        let ours = field
            .iter()
            .find(|candidate| candidate.who == self.us)
            .map(|candidate| worth_a_handover(&candidate.facts));
        ours.is_none_or(|ours| worth_a_handover(&winner.facts) > ours)
    }

    fn step_down_if_hosting(&mut self, now_ms: u64) -> Verdict {
        match self.standing {
            Standing::Hosting { since_ms } if now_ms.saturating_sub(since_ms) < HOLD_DOWN_MS => {
                Verdict::CarryOn
            }
            Standing::Hosting { .. } => {
                self.standing = Standing::Watching;
                Verdict::StandDown
            }
            _ => {
                self.standing = Standing::Watching;
                Verdict::CarryOn
            }
        }
    }
}

#[cfg(test)]
mod tests;
