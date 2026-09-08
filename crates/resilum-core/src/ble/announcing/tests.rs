use tokio::sync::mpsc;

use std::sync::Mutex;

use super::{put_us_on_the_air, what_we_put_on_the_air};
use crate::ble::beacon::{Beacon, ON_THE_AIR_LEN};
use crate::ble::radio::{ConnectionId, Outbound, PeerAddress, Radio, RadioError, RadioEvent};
use crate::ble::spec;

#[derive(Default)]
struct Whose {
    name_is_ours: bool,
    said: Mutex<Vec<&'static str>>,
}

impl Whose {
    fn with_a_name_of_its_own(name_is_ours: bool) -> Self {
        Self {
            name_is_ours,
            said: Mutex::new(Vec::new()),
        }
    }

    fn said(&self) -> Vec<&'static str> {
        self.said.lock().expect("held").clone()
    }
}

impl Radio for Whose {
    fn events_taken_once(&self) -> Option<mpsc::Receiver<RadioEvent>> {
        None
    }
    fn advertise(&self, _: &str, _: &[u8], _: u128) -> Result<(), RadioError> {
        self.said.lock().expect("held").push("advertise");
        Ok(())
    }
    fn stop_advertising(&self) {
        self.said.lock().expect("held").push("stop");
    }
    fn the_local_name_is_ours_to_spend(&self) -> bool {
        self.name_is_ours
    }
    fn scan(&self, _: u128) -> Result<(), RadioError> {
        Ok(())
    }
    fn stop_scan(&self) {}
    fn connect(&self, _: &PeerAddress) -> Result<(), RadioError> {
        Ok(())
    }
    fn disconnect(&self, _: ConnectionId) {}
    fn outbound_awaits_a_slot(&self) -> mpsc::Sender<Outbound> {
        mpsc::channel(1).0
    }
    fn read(&self, _: ConnectionId, _: u128) -> Result<(), RadioError> {
        Ok(())
    }
    fn bytes_one_write_carries(&self, _: ConnectionId) -> usize {
        20
    }
    fn serve_identity(&self, _: [u8; spec::IDENTITY_LEN]) {}
}

#[test]
fn a_radio_whose_name_is_ours_spends_the_name() {
    let beacon = Beacon::fresh(true, true);

    let air = what_we_put_on_the_air(&Whose::with_a_name_of_its_own(true), &beacon);

    assert_eq!(air.local_name, beacon.name());
    assert!(air.beacon.is_empty());
}

#[test]
fn a_radio_whose_name_belongs_to_the_user_spends_service_data() {
    let beacon = Beacon::fresh(true, true);

    let air = what_we_put_on_the_air(&Whose::with_a_name_of_its_own(false), &beacon);

    assert_eq!(air.local_name, "");
    assert_eq!(air.beacon.len(), ON_THE_AIR_LEN);
}

#[test]
fn neither_ever_spends_both_because_they_do_not_fit_together() {
    for the_name_is_ours in [true, false] {
        let air = what_we_put_on_the_air(
            &Whose::with_a_name_of_its_own(the_name_is_ours),
            &Beacon::fresh(true, true),
        );

        assert!(air.local_name.is_empty() || air.beacon.is_empty());
    }
}

#[test]
fn a_beacon_that_changed_replaces_the_one_on_the_air_rather_than_joining_it() {
    let radio = Whose::with_a_name_of_its_own(true);

    put_us_on_the_air(&radio, &Beacon::fresh(true, false)).expect("on the air");
    put_us_on_the_air(&radio, &Beacon::fresh(true, true)).expect("on the air again");

    assert_eq!(
        radio.said(),
        ["stop", "advertise", "stop", "advertise"],
        "a second advertisement registered beside the first is refused, \
         and the stale beacon stays on the air"
    );
}
