//! Service identifiers on the wire.
//!
//! One announce carries every transport a node offers, in a few hundred bytes,
//! so a service is a byte rather than its name. The values are permanent:
//! changing one silently points peers at the wrong transport, so a new service
//! takes a new number instead of a free-looking gap.

const SERVICES: [(&str, u8); 6] = [
    ("tor", 0x01),
    ("i2p", 0x02),
    ("yggdrasil", 0x03),
    ("iroh", 0x04),
    // Covert carriers sit above the transports that carry themselves.
    ("covert_icmp", 0x10),
    // Bridges to another protocol reuse the mesh's own announce rather than
    // a directory of their own.
    ("nostr_relay", 0x20),
];

/// A service this node can speak for. Only constructible by looking a name or
/// an id up in this module's wire table, so a name no peer could ever announce
/// cannot be registered as one — where a bare `&str` key let a typo compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Service {
    name: &'static str,
    id: u8,
}

impl Service {
    pub const TOR: Self = Self::at(0);
    pub const I2P: Self = Self::at(1);
    pub const YGGDRASIL: Self = Self::at(2);
    pub const IROH: Self = Self::at(3);
    pub const COVERT_ICMP: Self = Self::at(4);
    pub const NOSTR_RELAY: Self = Self::at(5);

    /// A name off the wire or out of a configuration file, or `None` when this
    /// build has no such service.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        SERVICES
            .iter()
            .position(|(known, _)| *known == name)
            .map(Self::at)
    }

    #[must_use]
    pub fn from_id(id: u8) -> Option<Self> {
        SERVICES
            .iter()
            .position(|(_, known)| *known == id)
            .map(Self::at)
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        self.name
    }

    #[must_use]
    pub fn id(self) -> u8 {
        self.id
    }

    const fn at(index: usize) -> Self {
        let (name, id) = SERVICES[index];
        Self { name, id }
    }
}

pub fn id_of(service: &str) -> Option<u8> {
    Service::from_name(service).map(Service::id)
}

pub fn name_of(id: u8) -> Option<&'static str> {
    Service::from_id(id).map(Service::name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_round_trips_through_its_id() {
        for (name, _) in SERVICES {
            let id = id_of(name).expect("service has an id");
            assert_eq!(name_of(id), Some(name));
        }
    }

    #[test]
    fn a_service_this_node_does_not_speak_has_no_id() {
        assert_eq!(id_of("nostr"), None);
        assert_eq!(id_of("covert_dns"), None);
        assert_eq!(name_of(0xfe), None);
        assert_eq!(Service::from_name("covert_dns"), None);
    }

    /// The named constants index [`SERVICES`] positionally, so a row inserted
    /// above one silently repoints it at its neighbour.
    #[test]
    fn each_named_constant_still_points_at_the_service_it_is_named_for() {
        for (service, name) in [
            (Service::TOR, "tor"),
            (Service::I2P, "i2p"),
            (Service::YGGDRASIL, "yggdrasil"),
            (Service::IROH, "iroh"),
            (Service::COVERT_ICMP, "covert_icmp"),
            (Service::NOSTR_RELAY, "nostr_relay"),
        ] {
            assert_eq!(service.name(), name);
            assert_eq!(Service::from_name(name), Some(service));
        }
    }
}
