//! What a restart must still remember: a subscription, its resume mark, and
//! the batch it landed in — including one a prior version wrote with no batch.

use super::*;

const WINDOW: Duration = Duration::from_secs(600);

#[test]
fn a_subscription_outlives_the_process_that_took_it() {
    let dir = scratch("restart");
    let path = dir.join("registry");

    let registry = Registry::open(path.clone(), WINDOW).expect("opens");
    registry.accept(sub(100, 0xcc), 100).expect("accepted");
    drop(registry);

    let reopened = Registry::open(path, WINDOW).expect("reopens");
    assert_eq!(reopened.lxmf_for(&[7u8; 32]).map(|a| a[15]), Some(0xcc));

    std::fs::remove_dir_all(&dir).ok();
}

/// A restart that resumed from a stale mark would re-fetch and re-deliver
/// every event between the two, to a subscriber that already has them.
#[test]
fn a_resume_mark_outlives_the_process_that_moved_it() {
    let dir = scratch("mark");
    let path = dir.join("registry");

    let registry = Registry::open(path.clone(), WINDOW).expect("opens");
    registry.accept(sub(100, 0xcc), 100).expect("accepted");
    registry.mark_seen(&[7u8; 32], 555);
    drop(registry);

    let reopened = Registry::open(path, WINDOW).expect("reopens");
    assert_eq!(reopened.last_seen(&[7u8; 32]), Some(555));

    std::fs::remove_dir_all(&dir).ok();
}

/// An expiry that never reached disk would come back on the next start, and
/// the bridge would resume asking relays for a subscriber that had lapsed.
#[test]
fn an_expiry_outlives_the_process_that_swept_it() {
    let dir = scratch("expire");
    let path = dir.join("registry");

    let registry = Registry::open(path.clone(), Duration::from_secs(60)).expect("opens");
    registry.accept(sub(100, 0xcc), 100).expect("accepted");
    registry.accept(nth(9, 100), 100).expect("accepted");
    assert_eq!(registry.expire(180), 2);
    drop(registry);

    let reopened = Registry::open(path, Duration::from_secs(60)).expect("reopens");
    assert_eq!(reopened.lxmf_for(&[7u8; 32]), None);
    assert_eq!(reopened.lxmf_for(&[9u8; 32]), None);

    std::fs::remove_dir_all(&dir).ok();
}

/// A batch assignment that changed across a restart would defeat the point:
/// the frame it rides in would be reissued for no subscriber-visible reason.
#[test]
fn a_subscribers_batch_survives_a_restart() {
    let dir = scratch("batch");
    let path = dir.join("registry");

    let registry = Registry::open(path.clone(), WINDOW).expect("opens");
    for pubkey in 0..FILTERS_PER_REQUEST as u8 {
        registry
            .accept(nth(pubkey, 100), 100)
            .expect("fills batch 0");
    }
    registry
        .accept(nth(FILTERS_PER_REQUEST as u8, 100), 100)
        .expect("opens batch 1");
    drop(registry);

    let reopened = Registry::open(path, WINDOW).expect("reopens");
    assert_eq!(
        reopened.batch_of(&[FILTERS_PER_REQUEST as u8; 32]),
        Some(BatchId::from_stored(1))
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_record_with_no_slot_is_migrated_and_the_file_is_rewritten() {
    use data_encoding::HEXLOWER;

    let dir = scratch("migrate");
    let path = dir.join("registry");
    let legacy_line = serde_json::json!({
        "pubkey": HEXLOWER.encode(&[7u8; 32]),
        "lxmf": HEXLOWER.encode(&[0x11u8; 16]),
        "created_at": 100,
        "last_seen": 100,
    });
    std::fs::write(&path, format!("{legacy_line}\n")).expect("seed a legacy line");

    let registry = Registry::open(path.clone(), WINDOW).expect("opens");
    assert_eq!(registry.batch_of(&[7u8; 32]), Some(BatchId::FIRST));
    drop(registry);

    let rewritten = std::fs::read_to_string(&path).expect("rewritten");
    assert!(rewritten.contains(r#""batch":0"#), "{rewritten}");

    std::fs::remove_dir_all(&dir).ok();
}

/// A directory stands in for the unreadable file: reading one fails on every
/// platform, and it needs neither a permission change nor a user the change
/// would not apply to.
#[test]
fn a_registry_file_that_cannot_be_read_refuses_to_open() {
    let dir = scratch("unreadable");
    let path = dir.join("registry");
    std::fs::create_dir_all(&path).expect("a directory where the file belongs");

    assert!(Registry::open(path, WINDOW).is_err());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_registry_file_that_does_not_exist_yet_opens_empty() {
    let dir = scratch("absent");

    let registry = Registry::open(dir.join("registry"), WINDOW).expect("opens");
    assert!(registry.live(100).is_empty());

    std::fs::remove_dir_all(&dir).ok();
}
