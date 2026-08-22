use std::collections::VecDeque;
use std::time::Duration;

const SEEN_WITHIN: f64 = 20.0 * 60.0;
const AT_MOST: usize = 32;
const SAMPLES_A_PATH_MUST_CARRY_ONE_OF: usize = 4;

struct Seen {
    rtt: Duration,
    at: f64,
}

#[derive(Default)]
pub(super) struct Window {
    seen: VecDeque<Seen>,
}

impl Window {
    pub(super) fn measured(&mut self, rtt: Duration, now: f64) {
        self.seen.push_back(Seen { rtt, at: now });
        self.seen.retain(|seen| now - seen.at <= SEEN_WITHIN);
        while self.seen.len() > AT_MOST {
            self.seen.pop_front();
        }
    }

    pub(super) fn dependably_fast(&self) -> Option<Duration> {
        let mut sorted: Vec<Duration> = self.seen.iter().map(|seen| seen.rtt).collect();
        sorted.sort_unstable();
        sorted.get(quarter_way_up(sorted.len())).copied()
    }
}

fn quarter_way_up(count: usize) -> usize {
    count
        .div_ceil(SAMPLES_A_PATH_MUST_CARRY_ONE_OF)
        .saturating_sub(1)
}

#[cfg(test)]
mod tests;
