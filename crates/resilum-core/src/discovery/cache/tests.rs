use super::*;

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
    let mut records = vec![
        Record {
            endpoint: hex_encode(b"old"),
            first_seen: 0.0,
            last_seen: 50.0,
        },
        Record {
            endpoint: hex_encode(b"new"),
            first_seen: 90.0,
            last_seen: 95.0,
        },
    ];
    let removed = prune(&mut records, 20.0, 100.0);
    assert_eq!(removed, 1);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].endpoint, hex_encode(b"new"));
}

#[test]
fn top_n_orders_by_recency() {
    let records = vec![
        Record {
            endpoint: hex_encode(b"a"),
            first_seen: 0.0,
            last_seen: 100.0,
        },
        Record {
            endpoint: hex_encode(b"b"),
            first_seen: 0.0,
            last_seen: 300.0,
        },
        Record {
            endpoint: hex_encode(b"c"),
            first_seen: 0.0,
            last_seen: 200.0,
        },
    ];
    let top = top_n(&records, 2);
    assert_eq!(top, vec![b"b".to_vec(), b"c".to_vec()]);
}

#[test]
fn save_then_load_roundtrips() {
    let dir = std::env::temp_dir().join(format!("resilum-cache-{}", std::process::id()));
    let path = dir.join("test.json");
    let records = vec![Record {
        endpoint: hex_encode(b"onion.onion:4242"),
        first_seen: 1.0,
        last_seen: 2.0,
    }];
    save(&path, &records).unwrap();
    let loaded = load(&path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].endpoint, records[0].endpoint);
    std::fs::remove_dir_all(&dir).ok();
}
