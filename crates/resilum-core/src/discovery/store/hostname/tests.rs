use super::*;

fn a_file_of_our_own() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("hostname");
    (dir, path)
}

#[test]
fn an_address_is_read_from_the_file_once_and_answered_from_memory_after() {
    let (_dir, path) = a_file_of_our_own();
    resilum_store::write_text(&path, "peer.onion\n").expect("an address to read");
    let advertised = Advertised::at(Some(path.clone()));
    assert_eq!(advertised.read().as_deref(), Some("peer.onion"));

    resilum_store::forget(&path).expect("the file goes away");

    assert_eq!(advertised.read().as_deref(), Some("peer.onion"));
}

#[test]
fn a_file_that_is_not_there_yet_is_read_again_next_time() {
    let (_dir, path) = a_file_of_our_own();
    let advertised = Advertised::at(Some(path.clone()));
    assert!(advertised.read().is_none());

    resilum_store::write_text(&path, "peer.onion\n").expect("an address to read");

    assert_eq!(advertised.read().as_deref(), Some("peer.onion"));
}

#[test]
fn forgetting_takes_the_file_with_it_so_a_dead_service_advertises_nothing() {
    let (_dir, path) = a_file_of_our_own();
    resilum_store::write_text(&path, "peer.onion\n").expect("an address to read");
    let advertised = Advertised::at(Some(path.clone()));
    assert!(advertised.read().is_some());

    advertised.forget();

    assert!(advertised.read().is_none());
    assert!(!path.exists());
}
