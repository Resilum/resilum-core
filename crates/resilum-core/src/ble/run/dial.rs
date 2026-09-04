use std::collections::HashMap;

use super::Ours;
use crate::ble::beacon::Beacon;
use crate::ble::direction::{Toward, toward};
use crate::ble::links::Links;
use crate::ble::radio::PeerAddress;

pub(super) const HOLD_OFF_MS: u64 = 4_000;

pub(super) type HeldOff = HashMap<PeerAddress, u64>;

pub(super) fn on_seen(
    ours: &Ours,
    links: &Links,
    held_off: &mut HeldOff,
    address: PeerAddress,
    name: Option<&str>,
    now_ms: u64,
) {
    let theirs = name.and_then(Beacon::read);
    if theirs.is_some_and(|theirs| theirs.group_is_up) {
        ours.someone_elses_group.heard_at(now_ms);
    }
    if !links.room_for_one_more() || held_off.contains_key(&address) {
        return;
    }
    match toward(&ours.beacon, theirs.as_ref()) {
        Toward::DialNow => {
            let _ = ours.radio.connect(&address);
        }
        Toward::LetThemDialFirst => {
            held_off.insert(address, now_ms);
        }
    }
}

pub(super) fn those_who_waited(ours: &Ours, links: &Links, held_off: &mut HeldOff, now_ms: u64) {
    let ready: Vec<PeerAddress> = held_off
        .iter()
        .filter(|(_, since)| now_ms.saturating_sub(**since) >= HOLD_OFF_MS)
        .map(|(address, _)| address.clone())
        .collect();
    for address in ready {
        held_off.remove(&address);
        if links.room_for_one_more() {
            let _ = ours.radio.connect(&address);
        }
    }
}
