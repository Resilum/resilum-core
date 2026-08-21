use std::collections::VecDeque;
use std::time::Duration;

const SEEN_WITHIN: f64 = 20.0 * 60.0;
const AT_MOST: usize = 32;

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

    pub(super) fn least(&self) -> Option<Duration> {
        self.seen.iter().map(|seen| seen.rtt).min()
    }
}

#[cfg(test)]
mod tests;
