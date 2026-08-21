use std::collections::VecDeque;
use std::time::Duration;

const SAMPLES: usize = 8;

#[derive(Default)]
pub(super) struct Window {
    seen: VecDeque<Duration>,
}

impl Window {
    pub(super) fn measured(&mut self, rtt: Duration) {
        self.seen.push_back(rtt);
        while self.seen.len() > SAMPLES {
            self.seen.pop_front();
        }
    }

    pub(super) fn least(&self) -> Option<Duration> {
        self.seen.iter().copied().min()
    }
}

#[cfg(test)]
mod tests;
