//! Effective address list for a covert carrier: explicit config wins;
//! otherwise, auto-detected globally-routable local egress addresses.

use crate::net;

pub struct AddressSource {
    configured: Vec<String>,
    detected: Vec<String>,
}

impl AddressSource {
    pub fn new(configured: Vec<String>) -> Self {
        let detected = if configured.is_empty() {
            net::local_egress_addresses()
                .into_iter()
                .map(|ip| ip.to_string())
                .collect()
        } else {
            Vec::new()
        };
        Self {
            configured,
            detected,
        }
    }

    pub fn effective(&self) -> &[String] {
        if !self.configured.is_empty() {
            &self.configured
        } else {
            &self.detected
        }
    }
}
