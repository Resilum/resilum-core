//! Announce payload: a msgpack envelope carrying every transport this node has
//! ready, and an exit country when it provides egress.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

/// Covers the envelope and the endpoint formats inside it: bump on a change to
/// either, and a peer on another version is dropped rather than half-read.
const WIRE_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAnnounce {
    /// Keyed by service name.
    pub endpoints: BTreeMap<String, Vec<u8>>,
    pub exit_country: String,
}

/// `[version, endpoints, country]` as a tuple, so the field names stay out of
/// the payload. Endpoints are keyed by [`crate::discovery::service`] id.
#[derive(Serialize, Deserialize)]
struct Wire(u8, BTreeMap<u8, ByteBuf>, Option<String>);

impl Wire {
    fn new(exit_country: &str) -> Self {
        let country =
            (exit_country != "*" && !exit_country.is_empty()).then(|| exit_country.to_owned());
        Self(WIRE_VERSION, BTreeMap::new(), country)
    }

    fn insert(&mut self, service: u8, endpoint: &[u8]) {
        self.1.insert(service, ByteBuf::from(endpoint.to_vec()));
    }

    fn remove(&mut self, service: u8) {
        self.1.remove(&service);
    }

    fn encode(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).expect("announce payload serializes")
    }
}

/// Services past `budget` are returned rather than packed: an oversized
/// announce is refused whole, taking down the transports that did fit.
pub fn pack(
    endpoints: &BTreeMap<String, Vec<u8>>,
    exit_country: &str,
    budget: usize,
) -> (Vec<u8>, Vec<String>) {
    let mut wire = Wire::new(exit_country);
    let mut left_out = Vec::new();
    // Smallest first, so a single fat endpoint costs itself rather than the
    // several small ones that would have fit in its place.
    let mut by_size: Vec<(&String, &Vec<u8>)> = endpoints.iter().collect();
    by_size.sort_by_key(|(service, bytes)| (bytes.len(), (*service).clone()));
    for (service, bytes) in by_size {
        let Some(id) = crate::discovery::service::id_of(service) else {
            left_out.push(service.clone());
            continue;
        };
        wire.insert(id, bytes);
        if wire.encode().len() > budget {
            wire.remove(id);
            left_out.push(service.clone());
        }
    }
    left_out.sort();
    (wire.encode(), left_out)
}

/// `None` means drop the peer: empty, malformed, or another wire version.
pub fn parse(raw: &[u8]) -> Option<ParsedAnnounce> {
    if raw.is_empty() {
        return None;
    }
    let Wire(version, eps, co) = rmp_serde::from_slice(raw).ok()?;
    if version != WIRE_VERSION {
        return None;
    }
    Some(ParsedAnnounce {
        endpoints: eps
            .into_iter()
            .filter_map(|(id, bytes)| {
                let service = crate::discovery::service::name_of(id)?;
                Some((service.to_owned(), bytes.into_vec()))
            })
            .collect(),
        exit_country: co.filter(|c| !c.is_empty()).unwrap_or_else(|| "*".into()),
    })
}

#[cfg(test)]
mod tests;
