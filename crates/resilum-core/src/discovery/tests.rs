use std::sync::Mutex;

use super::*;

#[derive(Default)]
struct Fake {
    consumed: Mutex<Vec<Vec<u8>>>,
}

impl DiscoveryPlugin for Fake {
    fn produce_endpoint(&self) -> Option<Vec<u8>> {
        Some(b"endpoint".to_vec())
    }
    fn consume_endpoint(&self, payload: &[u8], _announcer_pubkey: Option<&[u8]>) {
        self.consumed.lock().unwrap().push(payload.to_vec());
    }
}

fn announced(pairs: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    pairs
        .iter()
        .map(|(s, e)| ((*s).to_owned(), e.to_vec()))
        .collect()
}

#[test]
fn each_endpoint_reaches_the_plugin_for_its_service() {
    let plugin = Arc::new(Fake::default());
    let mut d = Discovery::default();
    d.register(Service::TOR, plugin.clone());

    // One announce, two services: the one nothing is registered for is
    // another node's transport, not an error.
    d.on_announce(
        &announced(&[("tor", b"payload"), ("i2p", b"other")]),
        b"pubkey",
    );

    assert_eq!(
        plugin.consumed.lock().unwrap().as_slice(),
        &[b"payload".to_vec()]
    );
}

/// A name outside the wire vocabulary cannot be registered, so the only way it
/// can arrive is in a peer's announce — where it must be ignored, not matched
/// against some neighbouring plugin.
#[test]
fn an_endpoint_naming_an_unknown_service_reaches_nobody() {
    let plugin = Arc::new(Fake::default());
    let mut d = Discovery::default();
    d.register(Service::TOR, plugin.clone());

    d.on_announce(&announced(&[("tro", b"typo")]), b"pubkey");

    assert!(plugin.consumed.lock().unwrap().is_empty());
}

#[test]
fn endpoints_lists_registered_services() {
    let mut d = Discovery::default();
    d.register(Service::TOR, Arc::new(Fake::default()));
    assert_eq!(d.endpoints(), announced(&[("tor", b"endpoint")]));
}
