//! What the loop does to a round: open one, join one, count a verdict into
//! one, and close the ones nothing answered.

use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::sync::Arc;
use std::time::Instant;

use data_encoding::HEXLOWER;
use serde_json::Value;

use super::round::Verdict;
use super::settle::settle;
use super::{Pending, Publication, Publishing, Source, accept};
use crate::bridge::tie::Tie;
use crate::queue::Entry;

impl Pending {
    pub(in crate::bridge) fn offer(
        &mut self,
        bridge: &Publishing<'_>,
        data: &Value,
        source: Source,
        now: Instant,
    ) {
        let Some(offered) = accept::offered(bridge, data, source) else {
            return;
        };
        if self.already_in_flight(offered.tie, Some(source.address())) {
            return;
        }
        let expected = (bridge.broadcast)(&offered.frame);
        let round = Publication::new(
            offered.tie,
            offered.event_json,
            vec![source.address()],
            expected,
            now,
        );
        self.start(bridge, round);
    }

    /// Returns whether anything actually went out: an attempt charged for a
    /// round that never opened walks the backoff to its cap over a relay
    /// outage without a frame having left the bridge.
    #[must_use]
    pub(in crate::bridge) fn republish(
        &mut self,
        bridge: &Publishing<'_>,
        entry: &Entry,
        now: Instant,
    ) -> bool {
        let tie = Tie::from(entry);
        if self.already_in_flight(tie, None) {
            return false;
        }
        let Some(frame) = accept::reframed(bridge, entry) else {
            return false;
        };
        let expected = (bridge.broadcast)(&frame);
        if expected == 0 {
            return false;
        }
        let json = Arc::clone(&entry.event_json);
        self.start(
            bridge,
            Publication::new(tie, json, Vec::new(), expected, now),
        );
        true
    }

    /// Whether this event is already in flight, in which case `reply_to`
    /// joins the round in progress: the relays' answer is one answer, and a
    /// second round against the same relays would only wait for copies of it.
    #[must_use]
    fn already_in_flight(&mut self, tie: Tie, reply_to: Option<[u8; 16]>) -> bool {
        let Some(round) = self.awaiting.get_mut(&HEXLOWER.encode(&tie.event_id)) else {
            return false;
        };
        if let Some(address) = reply_to {
            round.owed_to(address);
        }
        true
    }

    /// An event offered to no relay at all is answered here rather than left
    /// in a map nothing will ever call a verdict on.
    fn start(&mut self, bridge: &Publishing<'_>, round: Publication) {
        let event_id = HEXLOWER.encode(&round.tie.event_id);
        if round.answered() {
            return settle(bridge, &event_id, round);
        }
        match self.awaiting.entry(event_id) {
            // Merged rather than overwritten: a round dropped here would owe
            // acknowledgements nothing would ever send.
            Occupied(mut live) => live.get_mut().owed_to_each(&round.reply_to),
            Vacant(slot) => {
                slot.insert(round);
            }
        }
    }

    pub(in crate::bridge) fn verdict(
        &mut self,
        bridge: &Publishing<'_>,
        event_id: &str,
        verdict: Verdict,
    ) {
        let Some(round) = self.awaiting.get_mut(event_id) else {
            return;
        };
        round.counted(verdict);
        if !round.answered() {
            return;
        }
        if let Some(round) = self.awaiting.remove(event_id) {
            settle(bridge, event_id, round);
        }
    }

    /// A relay that never answers must not leave a sender waiting for it.
    pub(in crate::bridge) fn settle_expired(&mut self, bridge: &Publishing<'_>) {
        self.expire(bridge, Instant::now());
    }

    pub(super) fn expire(&mut self, bridge: &Publishing<'_>, now: Instant) {
        let out_of_time: Vec<String> = self
            .awaiting
            .iter()
            .filter(|(_, round)| round.deadline <= now)
            .map(|(event_id, _)| event_id.clone())
            .collect();
        for event_id in out_of_time {
            if let Some(round) = self.awaiting.remove(&event_id) {
                settle(bridge, &event_id, round);
            }
        }
    }
}
