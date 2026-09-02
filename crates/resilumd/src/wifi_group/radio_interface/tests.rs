use std::path::{Path, PathBuf};

use super::chosen_from;

fn devices_named(names: &[(&str, bool)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "resilum-radio-{}-{}",
        std::process::id(),
        names.len()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (name, wireless) in names {
        let device = root.join(name);
        std::fs::create_dir_all(&device).expect("device");
        if *wireless {
            std::fs::create_dir_all(device.join("phy80211")).expect("phy");
        }
    }
    root
}

#[test]
fn a_host_with_one_radio_needs_nothing_named() {
    let devices = devices_named(&[("eth0", false), ("wlan0", true)]);

    assert_eq!(chosen_from(&devices, None), Ok(String::from("wlan0")));
}

#[test]
fn a_wired_only_host_says_so_rather_than_guessing() {
    let devices = devices_named(&[("eth0", false)]);

    assert!(chosen_from(&devices, None).is_err());
}

#[test]
fn a_host_with_two_radios_asks_which_one() {
    let devices = devices_named(&[("wlan0", true), ("wlan1", true), ("eth0", false)]);

    assert!(chosen_from(&devices, None).is_err());
    assert_eq!(
        chosen_from(&devices, Some("wlan1")),
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
