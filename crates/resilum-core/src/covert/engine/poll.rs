//! Adaptive poll interval: fast under load, exponential backoff when idle.

use std::time::Duration;

pub struct AdaptivePoll {
    min: Duration,
    max: Duration,
    factor: u32,
    interval: Duration,
}

impl AdaptivePoll {
    pub fn new(min: Duration, max: Duration) -> Self {
        Self::with_factor(min, max, 2)
    }

    pub fn with_factor(min: Duration, max: Duration, factor: u32) -> Self {
        Self {
            min,
            max,
            factor,
            interval: min,
        }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn observe(&mut self, had_traffic: bool) {
        if had_traffic {
            self.interval = self.min;
        } else {
            self.interval = (self.interval * self.factor).min(self.max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traffic_resets_to_min_idle_backs_off_to_max() {
        let mut p = AdaptivePoll::new(Duration::from_millis(100), Duration::from_secs(1));
        assert_eq!(p.interval(), Duration::from_millis(100));
        p.observe(false);
        assert_eq!(p.interval(), Duration::from_millis(200));
        p.observe(false);
        assert_eq!(p.interval(), Duration::from_millis(400));
        p.observe(false);
        assert_eq!(p.interval(), Duration::from_millis(800));
        p.observe(false);
        assert_eq!(p.interval(), Duration::from_secs(1));
        p.observe(true);
        assert_eq!(p.interval(), Duration::from_millis(100));
    }
}
