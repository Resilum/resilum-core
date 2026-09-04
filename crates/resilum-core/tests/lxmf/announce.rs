use leviculum_core::node::NodeCoreBuilder;
use leviculum_core::{Destination, Identity, InterfaceId, MemoryStorage, NodeCore, NodeEvent};
use leviculum_lxmf::propagation_client::PropagationTransport;
use leviculum_lxmf::router::{LxmfRouter, RouterConfig};
use leviculum_lxmf::{LxmfNode, LxmfNodeConfig, announce};
use leviculum_std::driver::StdClock;
use rand_core::OsRng;
use resilum_core::lxmf::poll::event_to_json;
use serde_json::Value;

type TestCore = NodeCore<OsRng, StdClock, MemoryStorage>;

const AN_INSTANT_NO_CLOCK_WOULD_RETURN: f64 = 1_700_000_000.5;

fn identity_from(seed: u8) -> Identity {
    let mut private = [0u8; 64];
    for (index, byte) in private.iter_mut().enumerate() {
        *byte = seed.wrapping_add(index as u8);
    }
    Identity::from_private_key_bytes(&private).expect("deterministic identity")
}

fn a_node_with_a_router() -> (LxmfRouter, TestCore) {
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

fn what_a_caller_polls_after_announcing(
    destination: &mut Destination,
    app_data: &[u8],
) -> Vec<Value> {
    let (mut router, mut core) = a_node_with_a_router();
    let packet = destination
        .announce(Some(app_data), &mut OsRng, 1_000, 1)
        .expect("announce packet");
    let mut packed = vec![0u8; packet.packed_size()];
    let length = packet.pack(&mut packed).expect("pack announce");
    let heard = core
        .handle_packet(InterfaceId(0), &packed[..length])
        .events
        .into_iter()
        .find(|event| matches!(event, NodeEvent::AnnounceReceived { .. }))
        .expect("the announce reached the node");
    router
        .handle_event(&mut core, &heard)
        .expect("handle announce")
        .events
        .iter()
        .filter_map(|event| event_to_json(event, AN_INSTANT_NO_CLOCK_WOULD_RETURN))
        .map(|json| serde_json::from_str(&json).expect("well-formed json"))
        .collect()
}

fn announces(events: &[Value]) -> Vec<&Value> {
    events.iter().filter(|v| v["type"] == "announce").collect()
}

#[test]
fn a_delivery_announce_carries_its_display_name_and_the_time_it_was_heard() {
    let mut a_contact = LxmfNode::delivery_destination(identity_from(70)).expect("a contact");
    let its_address = resilum_core::hex::encode(a_contact.hash().as_bytes().iter());

    let events = what_a_caller_polls_after_announcing(
        &mut a_contact,
        &announce::delivery(Some(b"Anna"), Some(9)),
    );

    let announced = announces(&events);
    assert_eq!(announced.len(), 1, "exactly one announce event: {events:?}");
    let source = announced[0]["source"].as_str().expect("source is a string");
    assert_eq!(source, its_address);
    assert_eq!(source.len(), 32, "an lxmf address is 16 bytes of hex");
    assert_eq!(announced[0]["display_name"], "Anna");
    assert_eq!(announced[0]["timestamp"], AN_INSTANT_NO_CLOCK_WOULD_RETURN);
}

#[test]
fn a_nameless_announce_carries_an_empty_display_name() {
    let mut a_contact = LxmfNode::delivery_destination(identity_from(71)).expect("a contact");

    let events =
        what_a_caller_polls_after_announcing(&mut a_contact, &announce::delivery(None, None));

    let announced = announces(&events);
    assert_eq!(announced.len(), 1, "exactly one announce event: {events:?}");
    assert_eq!(
        announced[0].get("display_name"),
        Some(&Value::String(String::new())),
    );
}

#[test]
fn the_same_announce_from_a_propagation_node_never_reaches_the_caller() {
    let mut not_a_contact =
        PropagationTransport::destination(identity_from(72)).expect("a propagation node");

    let events = what_a_caller_polls_after_announcing(
        &mut not_a_contact,
        &announce::delivery(Some(b"Anna"), Some(9)),
    );

    assert!(
        announces(&events).is_empty(),
        "a propagation node is not a contact: {events:?}",
    );
}
