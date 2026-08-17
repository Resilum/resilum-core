//! Batch-slot assignment: stable across a refresh, reused once freed.

use super::*;

#[test]
fn an_expired_slot_is_taken_by_the_next_newcomer_rather_than_appended() {
    let registry = Registry::ephemeral(Duration::from_secs(100));
    registry
        .accept(nth(0, 0), 0)
        .expect("ages out first, alone");
    for pubkey in 1..FILTERS_PER_REQUEST as u8 {
        registry
            .accept(nth(pubkey, 50), 50)
            .expect("fills the rest of batch 0");
    }
    registry
        .accept(nth(FILTERS_PER_REQUEST as u8, 50), 50)
        .expect("batch 0 is full, opens batch 1");
    assert_eq!(registry.expire(100), 1, "only the first has aged out");

    registry.accept(nth(200, 100), 100).expect("newcomer");

    assert_eq!(
        registry.batch_of(&[200u8; 32]),
        Some(BatchId::FIRST),
        "reused the freed slot instead of landing in batch 1"
    );
}

#[test]
fn a_batch_nobody_refreshed_is_open_again_before_expiry_sweeps_it() {
    let registry = Registry::ephemeral(Duration::from_secs(100));
    for pubkey in 0..FILTERS_PER_REQUEST as u8 {
        registry.accept(nth(pubkey, 0), 0).expect("fills batch 0");
    }

    registry.accept(nth(200, 100), 100).expect("newcomer");

    assert_eq!(
        registry.batch_of(&[200u8; 32]),
        Some(BatchId::FIRST),
        "a batch of lapsed records was counted as still full"
    );
}
