use std::collections::BTreeMap;
use std::time::Duration;

use super::LinkId;
use super::claimed::Claimed;
use super::window::Window;

#[derive(Default)]
pub(super) struct Peer {
    pub(super) claimed: Option<Claimed>,
    pub(super) last_heard: f64,
    over: BTreeMap<LinkId, Window>,
}

impl Peer {
    pub(super) fn measured(&mut self, over: LinkId, rtt: Duration) {
        self.over.entry(over).or_default().measured(rtt);
    }

    pub(super) fn forget_link(&mut self, over: LinkId) {
        self.over.remove(&over);
    }

    pub(super) fn fastest_link(&self) -> Option<Duration> {
        self.over.values().filter_map(Window::typical).min()
    }
}

#[cfg(test)]
mod tests;
