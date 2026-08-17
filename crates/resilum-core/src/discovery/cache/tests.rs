use super::*;

fn record(endpoint: &[u8], first_seen: f64, last_seen: f64) -> Record {
    Record {
        endpoint_hex: HEXLOWER.encode(endpoint),
        first_seen,
        last_seen,
    }
}

#[test]
fn upsert_refreshes_existing() {
    let mut records = Vec::new();
    upsert(&mut records, b"peer.onion:4242", 100.0);
    upsert(&mut records, b"peer.onion:4242", 200.0);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].first_seen, 100.0);
    assert_eq!(records[0].last_seen, 200.0);
}

#[test]
fn prune_drops_expired() {
    let mut records = vec![record(b"old", 0.0, 50.0), record(b"new", 90.0, 95.0)];
    let removed = prune(&mut records, 20.0, 100.0);
    assert_eq!(removed, 1);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].endpoint_hex, HEXLOWER.encode(b"new"));
}

#[test]
fn top_n_orders_by_recency() {
    let records = vec![
        record(b"a", 0.0, 100.0),
        record(b"b", 0.0, 300.0),
        record(b"c", 0.0, 200.0),
    ];
    let top = top_n(&records, 2);
    assert_eq!(top, vec![b"b".to_vec(), b"c".to_vec()]);
}

/// The field this key was renamed from. A deployed node meets one of these
/// exactly once, and must come up with an empty cache rather than fail.
#[test]
fn a_cache_written_by_the_previous_field_layout_reads_as_empty() {
    let dir = std::env::temp_dir().join(format!("resilum-cache-old-{}", std::process::id()));
    let path = dir.join("test.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &path,
        br#"[{"endpoint":"6f6c64","first_seen":1.0,"last_seen":2.0}]"#,
    )
    .unwrap();

    assert!(load(&path).is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn save_then_load_roundtrips() {
    let dir = std::env::temp_dir().join(format!("resilum-cache-{}", std::process::id()));
    let path = dir.join("test.json");
    let records = vec![record(b"onion.onion:4242", 1.0, 2.0)];
    save(&path, &records).unwrap();
    let loaded = load(&path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].endpoint_hex, records[0].endpoint_hex);
    std::fs::remove_dir_all(&dir).ok();
}
