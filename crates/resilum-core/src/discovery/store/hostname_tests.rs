use super::*;

fn a_file_of_our_own(named: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("resilum-hostname-{named}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    dir.join("hostname")
}

#[test]
fn an_address_is_read_from_the_file_once_and_answered_from_memory_after() {
    let path = a_file_of_our_own("once");
    std::fs::write(&path, "peer.onion\n").expect("an address to read");
    let advertised = Advertised::at(Some(path.clone()));
    assert_eq!(advertised.read().as_deref(), Some("peer.onion"));

    std::fs::remove_file(&path).expect("the file goes away");

    assert_eq!(advertised.read().as_deref(), Some("peer.onion"));
    std::fs::remove_dir_all(path.parent().expect("the directory we made")).ok();
}

#[test]
fn a_file_that_is_not_there_yet_is_read_again_next_time() {
    let path = a_file_of_our_own("later");
    let advertised = Advertised::at(Some(path.clone()));
    assert!(advertised.read().is_none());

    std::fs::write(&path, "peer.onion\n").expect("an address to read");

    assert_eq!(advertised.read().as_deref(), Some("peer.onion"));
    std::fs::remove_dir_all(path.parent().expect("the directory we made")).ok();
}

#[test]
fn forgetting_takes_the_file_with_it_so_a_dead_service_advertises_nothing() {
    let path = a_file_of_our_own("forget");
    std::fs::write(&path, "peer.onion\n").expect("an address to read");
    let advertised = Advertised::at(Some(path.clone()));
    assert!(advertised.read().is_some());

    advertised.forget();

    assert!(advertised.read().is_none());
    assert!(!path.exists());
    std::fs::remove_dir_all(path.parent().expect("the directory we made")).ok();
}
