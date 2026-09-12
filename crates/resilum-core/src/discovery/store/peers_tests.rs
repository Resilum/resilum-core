use super::*;

fn a_directory_of_our_own(named: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("resilum-peers-{named}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    dir.join("service.json")
}

#[test]
fn what_one_run_saw_the_next_one_dials() {
    let path = a_directory_of_our_own("roundtrip");
    let remembered = Peers::open(Some(path.clone()));
    remembered.seen(b"onion.onion:4242", 100.0);
    drop(remembered);

    let next_run = Peers::open(Some(path.clone()));

    assert_eq!(next_run.most_recent(), vec![b"onion.onion:4242".to_vec()]);
    std::fs::remove_dir_all(path.parent().expect("the directory we made")).ok();
}

#[test]
fn a_peer_not_seen_within_the_ttl_is_dropped_from_the_file_too() {
    let path = a_directory_of_our_own("stale");
    let first_run = Peers::open(Some(path.clone()));
    first_run.seen(b"gone.onion:4242", 0.0);
    drop(first_run);

    let next_run = Peers::open(Some(path.clone()));
    assert_eq!(next_run.most_recent().len(), 1);
    next_run.forget_stale(TTL_SECONDS + 1.0);
    drop(next_run);

    assert!(Peers::open(Some(path.clone())).most_recent().is_empty());
    std::fs::remove_dir_all(path.parent().expect("the directory we made")).ok();
}

#[test]
fn a_cache_written_by_the_field_name_we_have_since_renamed_reads_as_empty() {
    let path = a_directory_of_our_own("old-layout");
    std::fs::write(
        &path,
        br#"[{"endpoint":"6f6c64","first_seen":1.0,"last_seen":2.0}]"#,
    )
    .expect("a cache to read back");

    assert!(Peers::open(Some(path.clone())).most_recent().is_empty());
    std::fs::remove_dir_all(path.parent().expect("the directory we made")).ok();
}

#[test]
fn a_store_with_nowhere_to_write_still_remembers_within_the_run() {
    let remembered = Peers::open(None);

    remembered.seen(b"onion.onion:4242", 100.0);

    assert_eq!(remembered.most_recent(), vec![b"onion.onion:4242".to_vec()]);
}
