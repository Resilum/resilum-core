use std::time::Duration;

use serde_json::{Value, json};

use crate::registry::BatchId;
use crate::registry::{FILTERS_PER_REQUEST, Registry};
use crate::subscription::Subscription;

use super::*;

const KINDS: [u32; 2] = [1059, 4];

fn nth(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn registered(byte: u8, created_at: i64) -> Subscription {
    Subscription {
        pubkey: nth(byte),
        lxmf: [0u8; 16],
        created_at,
    }
}

/// A frame is `["REQ", id, filter…]`, so the filters start at index two.
fn parts(frame: &str) -> Vec<Value> {
    let value: Value = serde_json::from_str(frame).expect("a frame we built is valid json");
    value.as_array().expect("a REQ is an array").clone()
}

fn mark(batch: usize, byte: u8, last_seen: i64) -> LiveMark {
    LiveMark {
        batch: BatchId::from_stored(batch),
        pubkey: nth(byte),
        last_seen,
    }
}

/// The whole reason for batching: one subscription, and every subscriber on
/// it resumes from its own mark rather than a mark merged across the group.
#[test]
fn one_batch_becomes_one_frame_carrying_every_subscribers_own_resume_point() {
    let marks = [mark(0, 1, 500), mark(0, 2, 700), mark(0, 3, 900)];

    let frames = requests(&KINDS, &marks);

    assert_eq!(frames.len(), 1);
    let parts = parts(&frames[0]);
    assert_eq!(parts[0], json!("REQ"));
    assert_eq!(parts[2]["#p"], json!([HEXLOWER.encode(&nth(1))]));
    assert_eq!(parts[2]["since"], json!(500));
    assert_eq!(parts[3]["since"], json!(700));
    assert_eq!(parts[4]["since"], json!(900));
}

/// A mark's batch says which `REQ` it rides in; two different batches must
/// never be folded into one frame, or a relay-side reissue of one would
/// silently touch the other's subscribers too.
#[test]
fn marks_in_two_different_batches_become_two_frames() {
    let mut marks: Vec<LiveMark> = (0..FILTERS_PER_REQUEST as u8)
        .map(|byte| mark(0, byte, 100))
        .collect();
    marks.push(mark(1, FILTERS_PER_REQUEST as u8, 100));

    let frames = requests(&KINDS, &marks);

    assert_eq!(frames.len(), 2);
    let (first, second) = (parts(&frames[0]), parts(&frames[1]));
    assert_eq!(first.len() - 2, FILTERS_PER_REQUEST);
    assert_eq!(second.len() - 2, 1, "the one in the other batch");
    assert_ne!(
        first[1], second[1],
        "two subscriptions on one connection cannot share an id"
    );
}

#[test]
fn a_bridge_with_no_subscribers_asks_for_nothing() {
    assert!(requests(&KINDS, &[]).is_empty());
}

#[test]
fn accepting_a_new_subscriber_leaves_every_other_batchs_frame_byte_identical() {
    let registry = Registry::ephemeral(Duration::from_secs(600));
    for byte in 0..FILTERS_PER_REQUEST as u8 {
        registry
            .accept(registered(byte, 100), 100)
            .expect("fills batch 0");
    }
    registry
        .accept(registered(FILTERS_PER_REQUEST as u8, 100), 100)
        .expect("batch 0 is full, opens batch 1");
    let before = requests(&KINDS, &registry.live_marks(100));

    registry
        .accept(registered(FILTERS_PER_REQUEST as u8 + 1, 200), 200)
        .expect("batch 1 is the only one with room");
    let after = requests(&KINDS, &registry.live_marks(200));

    assert_eq!(before[0], after[0], "batch 0 took on no new member");
}

/// A refresh keeps its slot, so nothing about the frames should move by even
/// a byte — not just the batch the refreshed subscriber sits in.
#[test]
fn a_refresh_changes_no_frame_at_all() {
    let registry = Registry::ephemeral(Duration::from_secs(600));
    for byte in 0..3 {
        registry
            .accept(registered(byte, 100), 100)
            .expect("accepted");
    }
    let before = requests(&KINDS, &registry.live_marks(100));

    registry.accept(registered(0, 200), 200).expect("refresh");
    let after = requests(&KINDS, &registry.live_marks(200));

    assert_eq!(before, after);
}
