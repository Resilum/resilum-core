use super::*;

fn a_cache_of_our_own() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("service.json");
    (dir, path)
}

#[test]
fn what_one_run_saw_the_next_one_dials() {
    let (_dir, path) = a_cache_of_our_own();
    let remembered = Peers::open(Some(path.clone()));
    remembered.seen(b"onion.onion:4242", 100.0);
    drop(remembered);

    let next_run = Peers::open(Some(path));

    assert_eq!(next_run.most_recent(), vec![b"onion.onion:4242".to_vec()]);
}

#[test]
fn a_peer_not_seen_within_the_ttl_is_dropped_from_the_file_too() {
    let (_dir, path) = a_cache_of_our_own();
    let first_run = Peers::open(Some(path.clone()));
    first_run.seen(b"gone.onion:4242", 0.0);
    drop(first_run);

    let next_run = Peers::open(Some(path.clone()));
    assert_eq!(next_run.most_recent().len(), 1);
    next_run.forget_stale(TTL_SECONDS + 1.0);
    drop(next_run);

    assert!(Peers::open(Some(path)).most_recent().is_empty());
}

#[test]
fn a_cache_written_by_the_field_name_we_have_since_renamed_reads_as_empty() {
    let (_dir, path) = a_cache_of_our_own();
    resilum_store::write_text(
        &path,
        r#"[{"endpoint":"6f6c64","first_seen":1.0,"last_seen":2.0}]"#,
    )
    .expect("a cache to read back");

    assert!(Peers::open(Some(path)).most_recent().is_empty());
}

#[test]
fn a_store_with_nowhere_to_write_still_remembers_within_the_run() {
    let remembered = Peers::open(None);

    remembered.seen(b"onion.onion:4242", 100.0);

    assert_eq!(remembered.most_recent(), vec![b"onion.onion:4242".to_vec()]);
}
