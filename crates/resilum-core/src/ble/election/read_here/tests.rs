use super::{Facts, PowerHere, answered_with};

fn told_it_is_running_down() -> Facts {
    Facts {
        charging: false,
        battery_percent: 7,
        has_an_uplink: true,
        ..Facts::default()
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn a_full_battery_left_plugged_in_is_still_a_wall_socket() {
    for plugged_in in ["Charging", "Not charging", "Full", "Unknown"] {
        assert!(
            super::at_a_socket(plugged_in),
            "{plugged_in} was read as unplugged"
        );
    }
    assert!(!super::at_a_socket("Discharging"));
}

#[test]
fn a_host_that_cannot_read_its_own_power_keeps_every_word_it_was_told() {
    let told = told_it_is_running_down();

    assert_eq!(answered_with(PowerHere::NotOursToRead, told), told);
}

#[test]
fn a_host_with_no_battery_of_its_own_counts_as_mains_powered() {
    let answered = answered_with(PowerHere::NoBatteryAtAll, told_it_is_running_down());

    assert!(answered.charging);
    assert_eq!(answered.battery_percent, 100);
}

#[test]
fn a_host_that_read_its_own_power_believes_itself_over_what_it_was_told() {
    let answered = answered_with(
        PowerHere::ABattery {
            percent: 93,
            charging: true,
        },
        told_it_is_running_down(),
    );

    assert!(answered.charging);
    assert_eq!(answered.battery_percent, 93);
}

#[test]
fn what_only_the_platform_could_know_survives_whatever_the_power_says() {
    for power in [
        PowerHere::NotOursToRead,
        PowerHere::NoBatteryAtAll,
        PowerHere::ABattery {
            percent: 1,
            charging: false,
        },
    ] {
        assert!(answered_with(power, told_it_is_running_down()).has_an_uplink);
    }
}
