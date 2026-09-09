use std::time::Duration;

use serde_json::{Value, json};

use crate::registry::BatchId;
use crate::registry::Registry;
use crate::subscription::Subscription;

use super::*;

const KINDS: [u32; 2] = [GIFT_WRAP_KIND, 4];
const NOW: i64 = 1_787_164_439;

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

fn accept_until_a_second_batch_opens(registry: &Registry) -> u8 {
    for byte in 0..=u8::MAX {
        registry
            .accept(registered(byte, 100), 100)
            .expect("accepted");
        if requests(&KINDS, &registry.live_marks(100)).len() == 2 {
            return byte;
        }
    }
    panic!("a second batch never opened");
}

fn mark(batch: usize, byte: u8, last_seen: i64) -> LiveMark {
    LiveMark {
        batch: BatchId::from_stored(batch),
        pubkey: nth(byte),
        last_seen,
    }
}

#[test]
fn one_batch_becomes_one_frame_carrying_every_subscribers_own_resume_point() {
    let marks = [
        mark(0, 1, NOW + 500),
        mark(0, 2, NOW + 700),
        mark(0, 3, NOW + 900),
    ];

    let frames = requests(&KINDS, &marks);

    assert_eq!(frames.len(), 1);
    let parts = parts(&frames[0]);
    assert_eq!(parts[0], json!("REQ"));
    assert_eq!(parts[2]["#p"], json!([HEXLOWER.encode(&nth(1))]));
    assert_eq!(parts[2]["since"], json!(NOW + 500 - NIP59_BACKDATE));
    assert_eq!(parts[3]["since"], json!(NOW + 700 - NIP59_BACKDATE));
    assert_eq!(parts[4]["since"], json!(NOW + 900 - NIP59_BACKDATE));
}

#[test]
fn a_filter_asking_for_gift_wraps_resumes_from_before_the_backdate() {
    let plain = requests(&[4], &[mark(0, 1, NOW)]);
    let wrapped = requests(&KINDS, &[mark(0, 1, NOW)]);

    assert_eq!(parts(&plain[0])[2]["since"], json!(NOW));
    assert_eq!(parts(&wrapped[0])[2]["since"], json!(NOW - NIP59_BACKDATE));
}

/// Folded into one frame, a reissue of either batch would silently touch the
/// other's subscribers too.
#[test]
fn marks_in_two_different_batches_become_two_frames() {
    let mut marks: Vec<LiveMark> = (0..3).map(|byte| mark(0, byte, 100)).collect();
    marks.push(mark(1, 3, 100));

    let frames = requests(&KINDS, &marks);

    assert_eq!(frames.len(), 2);
    let (first, second) = (parts(&frames[0]), parts(&frames[1]));
    assert_eq!(first.len() - 2, 3);
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
    let opened_the_second_batch = accept_until_a_second_batch_opens(&registry);
    let before = requests(&KINDS, &registry.live_marks(100));

    registry
        .accept(registered(opened_the_second_batch + 1, 200), 200)
        .expect("batch 1 is the only one with room");
    let after = requests(&KINDS, &registry.live_marks(200));

    assert_eq!(before[0], after[0], "batch 0 took on no new member");
}

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
