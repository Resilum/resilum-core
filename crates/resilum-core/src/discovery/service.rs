//! Service identifiers on the wire.
//!
//! One announce carries every transport a node offers, in a few hundred bytes,
//! so a service is a byte rather than its name. The values are permanent:
//! changing one silently points peers at the wrong transport, so a new service
//! takes a new number instead of a free-looking gap.

const SERVICES: [(&str, u8); 5] = [
    ("tor", 0x01),
    ("i2p", 0x02),
    ("yggdrasil", 0x03),
    ("iroh", 0x04),
    // Covert carriers sit above the transports that carry themselves.
    ("covert_icmp", 0x10),
];

pub fn id_of(service: &str) -> Option<u8> {
    SERVICES
        .iter()
        .find(|(name, _)| *name == service)
        .map(|(_, id)| *id)
}

pub fn name_of(id: u8) -> Option<&'static str> {
    SERVICES
        .iter()
        .find(|(_, known)| *known == id)
        .map(|(name, _)| *name)
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
    }

    #[test]
    fn no_two_services_share_an_id() {
        let mut ids: Vec<u8> = SERVICES.iter().map(|(_, id)| *id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate service id");
    }
}
