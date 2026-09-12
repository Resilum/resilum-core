use std::path::Path;

use super::chosen_from;

fn devices_named(names: &[(&str, bool)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary directory");
    for (name, wireless) in names {
        let device = root.path().join(name);
        resilum_store::make_room_for(&device).expect("device");
        if *wireless {
            resilum_store::make_room_for(&device.join("phy80211")).expect("phy");
        }
    }
    root
}

#[test]
fn a_host_with_one_radio_needs_nothing_named() {
    let devices = devices_named(&[("eth0", false), ("wlan0", true)]);

    assert_eq!(chosen_from(devices.path(), None), Ok(String::from("wlan0")));
}

#[test]
fn a_wired_only_host_says_so_rather_than_guessing() {
    let devices = devices_named(&[("eth0", false)]);

    assert!(chosen_from(devices.path(), None).is_err());
}

#[test]
fn a_host_with_two_radios_asks_which_one() {
    let devices = devices_named(&[("wlan0", true), ("wlan1", true), ("eth0", false)]);

    assert!(chosen_from(devices.path(), None).is_err());
    assert_eq!(
        chosen_from(devices.path(), Some("wlan1")),
        Ok(String::from("wlan1"))
    );
}

#[test]
fn a_named_interface_is_taken_even_where_nothing_can_be_read() {
    assert_eq!(
        chosen_from(Path::new("/nowhere-at-all"), Some("ap0")),
        Ok(String::from("ap0"))
    );
}
