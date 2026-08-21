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

    pub(super) fn typical(&self) -> Option<Duration> {
        if self.seen.is_empty() {
            return None;
        }
        let mut sorted: Vec<Duration> = self.seen.iter().copied().collect();
        sorted.sort_unstable();
        sorted.get(sorted.len() / 2).copied()
    }
}

#[cfg(test)]
mod tests;
