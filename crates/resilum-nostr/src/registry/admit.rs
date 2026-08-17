//! Whether a subscription request is one to write down.

use super::batch::{BatchId, Placement};
use super::store::{Held, Record};
use crate::subscription::Subscription;

/// How many per-subscriber filters ride in one `REQ`.
///
/// NIP-01 lets a subscription carry several filters and returns an event
/// matching any of them, which is what lets subscribers share a subscription
/// without sharing a `since`. Relays cap the count: strfry's
/// `maxReqFilterSize` default is 200, nostream's `maxFilters` default is 10.
/// No relay surveyed advertises NIP-11's `max_filters` at all, so the figure
/// has to fit the smallest documented default rather than a measured one.
pub(crate) const FILTERS_PER_REQUEST: usize = 10;

/// How many subscriptions the bridge will hold open on one connection.
///
/// nostream's `maxSubscriptions` default is 10; deployed relays advertise
/// more (nos.lol and relay.primal.net 20, purplepag.es and nostr.wine 50,
/// relay.damus.io 200), so 10 is under every one of them. Publishing costs
/// no subscription — an `EVENT` is not a `REQ` — so the whole budget is
/// available for subscribers.
const REQUESTS_PER_CONNECTION: usize = 10;

/// How many subscribers one bridge will hold at once.
///
/// Not a disk figure: past this the bridge's own upstreams start refusing
/// its requests, one `REQ` at a time, and the subscriber who tips it over is
/// not the one who stops being served.
///
/// On a bridge with a populated `allow_npubs` the list is the real bound and
/// this ceiling is never approached. On one that admits everyone it is the
/// only bound, and a keypair costs nothing to mint, so a single peer can hold
/// every slot. What would bound that is a verified sender identity; the
/// bridge has one for publishing but `subscribe` deliberately does not use it.
pub(super) const MAX_SUBSCRIBERS: usize = FILTERS_PER_REQUEST * REQUESTS_PER_CONNECTION;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AcceptError {
    Lapsed,
    Replayed,
    Full { limit: usize },
}

impl std::fmt::Display for AcceptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lapsed => f.write_str("this subscription is older than this bridge keeps them"),
            Self::Replayed => f.write_str("a newer subscription is already on file"),
            Self::Full { limit } => write!(f, "this bridge already carries {limit} subscribers"),
        }
    }
}

impl std::error::Error for AcceptError {}

/// The replay guard below only fires while a record exists, so once `expire`
/// has dropped a lapsed subscriber a captured request for it reads as new
/// again, as does one for a pubkey this bridge has never held. The bound is
/// the window itself and nothing tighter because a request may sit in a
/// propagation mailbox for days before it reaches us.
pub(super) fn ensure_within_retention(
    sub: &Subscription,
    now: i64,
    retention: i64,
) -> Result<(), AcceptError> {
    if now - sub.created_at >= retention {
        return Err(AcceptError::Lapsed);
    }
    Ok(())
}

/// A request no newer than the record on file is a capture being replayed:
/// accepting it would point delivery back at an address its owner has left.
pub(super) fn ensure_not_replayed(held: &Held, sub: &Subscription) -> Result<(), AcceptError> {
    if held
        .get(&sub.pubkey)
        .is_some_and(|e| sub.created_at <= e.created_at)
    {
        return Err(AcceptError::Replayed);
    }
    Ok(())
}

/// A refresh of a subscriber already held is not a new one, and a subscriber
/// `live` has stopped counting holds no place either — the slot goes as soon
/// as it stops being served, not when `expire` next gets round to the disk.
pub(super) fn ensure_room_for(
    held: &Held,
    sub: &Subscription,
    now: i64,
    retention: i64,
) -> Result<(), AcceptError> {
    if held.contains_key(&sub.pubkey) {
        return Ok(());
    }
    if live_count(held, now, retention) >= MAX_SUBSCRIBERS {
        return Err(AcceptError::Full {
            limit: MAX_SUBSCRIBERS,
        });
    }
    Ok(())
}

/// The mark the new record carries: on a refresh the one already on file,
/// since a keepalive is not proof that anything has been fetched.
pub(super) fn carried_mark(held: &Held, sub: &Subscription, now: i64) -> i64 {
    held.get(&sub.pubkey).map_or(now, |e| e.last_seen)
}

fn live_count(held: &Held, now: i64, retention: i64) -> usize {
    held.values()
        .filter(|record| now - record.created_at < retention)
        .count()
}

pub(super) fn place(held: &Held, sub: &Subscription, now: i64, retention: i64) -> Placement {
    match held.get(&sub.pubkey) {
        Some(existing) => Placement::Refreshed(existing.batch),
        None => Placement::Joined(lowest_open(held, |record| {
            now - record.created_at < retention
        })),
    }
}

/// Lowest first, not least-loaded: filling batches in order is what keeps a
/// personal bridge, which never nears the cap, down to a single `REQ`.
///
/// `occupies` is all that differs between placing a fresh subscription and
/// migrating a file at load, and those two have to agree or a migrated file
/// can hold more than `FILTERS_PER_REQUEST` in one batch.
pub(in crate::registry) fn lowest_open(held: &Held, occupies: impl Fn(&Record) -> bool) -> BatchId {
    let mut batch = BatchId::FIRST;
    loop {
        let taken = held
            .values()
            .filter(|record| record.batch == batch && occupies(record))
            .count();
        if taken < FILTERS_PER_REQUEST {
            return batch;
        }
        batch = batch.next();
    }
}
