use resilum_core::WifiGroup;

use super::{Connection, a_group_owned_by_us, a_group_someone_else_owns};

fn text_in(connection: &Connection, section: &str, key: &str) -> Option<String> {
    connection
        .get(section)?
        .get(key)?
        .0
        .as_str()
        .map(str::to_owned)
}

#[test]
fn the_group_we_own_is_an_access_point_at_an_address_we_chose() {
    let ours = a_group_owned_by_us(&WifiGroup::default(), "wlan0");

    assert_eq!(
        text_in(&ours, "802-11-wireless", "mode").as_deref(),
        Some("ap")
    );
    assert_eq!(text_in(&ours, "ipv4", "method").as_deref(), Some("manual"));
}

#[test]
fn the_group_someone_else_owns_is_joined_as_a_station_on_their_terms() {
    let theirs = a_group_someone_else_owns(&WifiGroup::default(), "wlan0");

    assert_eq!(
        text_in(&theirs, "802-11-wireless", "mode").as_deref(),
        Some("infrastructure")
    );
    assert_eq!(text_in(&theirs, "ipv4", "method").as_deref(), Some("auto"));
}

#[test]
fn the_two_never_answer_to_the_same_name() {
    let ours = a_group_owned_by_us(&WifiGroup::default(), "wlan0");
    let theirs = a_group_someone_else_owns(&WifiGroup::default(), "wlan0");

    assert_ne!(
        text_in(&ours, "connection", "id"),
        text_in(&theirs, "connection", "id")
    );
}

#[test]
fn joining_carries_the_passphrase_the_group_was_raised_with() {
    let group = WifiGroup::default();

    let theirs = a_group_someone_else_owns(&group, "wlan0");

    assert_eq!(
        text_in(&theirs, "802-11-wireless-security", "psk").as_deref(),
        Some(group.passphrase.as_str())
    );
}
