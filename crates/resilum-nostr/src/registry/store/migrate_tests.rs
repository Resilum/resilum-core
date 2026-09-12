use std::time::Duration;

use data_encoding::HEXLOWER;

use crate::registry::{BatchId, Registry};

#[test]
fn a_record_with_no_slot_is_migrated_and_the_file_is_rewritten() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("registry");
    let legacy_line = serde_json::json!({
        "pubkey": HEXLOWER.encode(&[7u8; 32]),
        "lxmf": HEXLOWER.encode(&[0x11u8; 16]),
        "created_at": 100,
        "last_seen": 100,
    });
    resilum_store::write_text(&path, &format!("{legacy_line}\n")).expect("seed a legacy line");

    let registry = Registry::open(path.clone(), Duration::from_secs(600)).expect("opens");
    assert_eq!(registry.batch_of(&[7u8; 32]), Some(BatchId::FIRST));
    drop(registry);

    let rewritten = resilum_store::read_text(&path).expect("rewritten");
    assert!(rewritten.contains(r#""batch":0"#), "{rewritten}");
}
