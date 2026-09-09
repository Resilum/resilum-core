use super::beacon::Beacon;
use super::radio::{Radio, RadioError};
use super::spec;

pub struct OnTheAir {
    pub local_name: String,
    pub beacon: Vec<u8>,
}

#[must_use]
pub fn what_we_put_on_the_air(radio: &dyn Radio, beacon: &Beacon) -> OnTheAir {
    if radio.the_local_name_is_ours_to_spend() {
        OnTheAir {
            local_name: beacon.name(),
            beacon: Vec::new(),
        }
    } else {
        OnTheAir {
            local_name: String::new(),
            beacon: beacon.on_the_air().to_vec(),
        }
    }
}

pub fn put_us_on_the_air(radio: &dyn Radio, beacon: &Beacon) -> Result<(), RadioError> {
    radio.stop_advertising();
    let air = what_we_put_on_the_air(radio, beacon);
    radio.advertise(&air.local_name, &air.beacon, spec::SERVICE)
}

#[cfg(test)]
mod tests;
