use std::path::PathBuf;

use resilum_core::WifiGroup;

use super::{describing, named_under, the_one_not_there_before};

fn sockets_called(test: &str, names: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("resilum-wpa-dir-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("control directory");
    for name in names {
        std::fs::write(root.join(name), b"").expect("socket");
    }
    root
}

#[test]
fn the_group_we_just_raised_wins_over_one_that_was_already_there() {
    let before = [String::from("p2p-wlan0-9")];
    let now = vec![String::from("p2p-wlan0-9"), String::from("p2p-wlan0-0")];

    assert_eq!(
        the_one_not_there_before(now, &before),
        Some(String::from("p2p-wlan0-0"))
    );
}

#[test]
fn nothing_new_means_no_group_to_carry() {
    let before = [String::from("p2p-wlan0-0")];
    let now = vec![String::from("p2p-wlan0-0")];

    assert_eq!(the_one_not_there_before(now, &before), None);
}

#[test]
fn the_device_socket_is_not_mistaken_for_a_group() {
    let sockets = sockets_called(
        "device-is-not-a-group",
        &["wlan0", "p2p-dev-wlan0", "p2p-wlan0-0"],
    );

    assert_eq!(
        named_under(&sockets, "p2p-wlan0-"),
        vec![String::from("p2p-wlan0-0")]
    );
}

#[test]
fn we_describe_a_persistent_group_owner_and_not_an_access_point() {
    let group = WifiGroup::default();

    let block = describing(&group);
    let value_of = |key: &str| {
        block
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.clone())
    };

    assert_eq!(value_of("mode"), Some(String::from("3")));
    assert_eq!(value_of("disabled"), Some(String::from("2")));
    assert_eq!(
        value_of("ssid"),
        Some(format!("\"{}\"", WifiGroup::default().ssid))
    );
}
