//! What an `lxmf.delivery` announce becomes on the delivery queue, driven from
//! a real announce packet rather than a hand-built router event: the aspect
//! filter this contract rests on lives between the two.

use leviculum_core::node::NodeCoreBuilder;
use leviculum_core::{Destination, Identity, InterfaceId, MemoryStorage, NodeCore, NodeEvent};
use leviculum_lxmf::propagation_client::PropagationTransport;
use leviculum_lxmf::router::{LxmfRouter, RouterConfig};
use leviculum_lxmf::{LxmfNode, LxmfNodeConfig, announce};
use leviculum_std::driver::StdClock;
use rand_core::OsRng;
use serde_json::Value;

use super::event_to_json;

type TestCore = NodeCore<OsRng, StdClock, MemoryStorage>;

fn identity_from(seed: u8) -> Identity {
    let mut private = [0u8; 64];
    for (index, byte) in private.iter_mut().enumerate() {
        *byte = seed.wrapping_add(index as u8);
    }
    Identity::from_private_key_bytes(&private).expect("deterministic identity")
}

/// A node with an LXMF router attached, as the processor builds one.
fn router_and_core() -> (LxmfRouter, TestCore) {
    let mut core =
        NodeCoreBuilder::new().build(OsRng, StdClock::new(), MemoryStorage::with_defaults());
    let identity = identity_from(1);
    let identity_hash = *identity.hash();
    let destination = LxmfNode::delivery_destination(identity).expect("delivery destination");
    let node = LxmfNode::register(&mut core, destination, LxmfNodeConfig::default())
        .expect("register delivery destination");
    (
        LxmfRouter::new(node, identity_hash, RouterConfig::default()),
        core,
    )
}

/// The instant `polled` claims to have heard the announce at. Arbitrary, and
/// nothing like a real reading, which is the point: a rendering that went to
/// the clock instead would never produce it.
const HEARD_AT: f64 = 1_700_000_000.5;

/// Announce `destination` onto a fresh node and return the events a caller
/// polling that node would see.
fn polled(destination: &mut Destination, app_data: &[u8]) -> Vec<Value> {
    let (mut router, mut core) = router_and_core();
    let packet = destination
        .announce(Some(app_data), &mut OsRng, 1_000, 1)
        .expect("announce packet");
    let mut packed = vec![0u8; packet.packed_size()];
    let length = packet.pack(&mut packed).expect("pack announce");
    let event = core
        .handle_packet(InterfaceId(0), &packed[..length])
        .events
        .into_iter()
        .find(|event| matches!(event, NodeEvent::AnnounceReceived { .. }))
        .expect("the announce reached the node");
    router
        .handle_event(&mut core, &event)
        .expect("handle announce")
        .events
        .iter()
        .filter_map(|event| event_to_json(event, HEARD_AT))
        .map(|json| serde_json::from_str(&json).expect("well-formed json"))
        .collect()
}

fn announces(events: &[Value]) -> Vec<&Value> {
    events.iter().filter(|v| v["type"] == "announce").collect()
}

/// The presence line a caller draws: an address it can message and a name to
/// put beside it, without the contact having sent anything.
#[test]
fn a_delivery_announce_carries_its_display_name_and_the_time_it_was_heard() {
    let identity = identity_from(70);
    let mut destination =
        LxmfNode::delivery_destination(identity).expect("remote delivery destination");
    let expected = crate::hex::encode(destination.hash().as_bytes().iter());

    let events = polled(
        &mut destination,
        &announce::delivery(Some(b"Anna"), Some(9)),
    );

    let announced = announces(&events);
    assert_eq!(announced.len(), 1, "exactly one announce event: {events:?}");
    let source = announced[0]["source"].as_str().expect("source is a string");
    assert_eq!(source, expected);
    assert_eq!(source.len(), 32, "an lxmf address is 16 bytes of hex");
    assert_eq!(announced[0]["display_name"], "Anna");
    assert_eq!(announced[0]["timestamp"], HEARD_AT);
}

#[test]
fn a_nameless_announce_carries_an_empty_display_name() {
    let identity = identity_from(71);
    let mut destination =
        LxmfNode::delivery_destination(identity).expect("remote delivery destination");

    let events = polled(&mut destination, &announce::delivery(None, None));

    let announced = announces(&events);
    assert_eq!(announced.len(), 1, "exactly one announce event: {events:?}");
    assert_eq!(
        announced[0].get("display_name"),
        Some(&Value::String(String::new())),
    );
}

/// The app data here is a perfectly valid delivery announce, so the only thing
/// that can reject this is the aspect the destination was built under. Feeding
/// a malformed blob instead would pass with the filter deleted.
#[test]
fn a_non_delivery_announce_never_reaches_the_caller() {
    let identity = identity_from(72);
    let mut destination =
        PropagationTransport::destination(identity).expect("propagation destination");

    let events = polled(
        &mut destination,
        &announce::delivery(Some(b"Anna"), Some(9)),
    );

    assert!(
        announces(&events).is_empty(),
        "a propagation node is not a contact: {events:?}",
    );
}
