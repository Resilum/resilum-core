#[path = "support/fake_radio/mod.rs"]
mod fake_radio;

use std::sync::Arc;
use std::time::Duration;

use fake_radio::{Air, CARRIED_PER_WRITE, FakeRadio};
use resilum_core::ble::election::{Candidate, Facts, HostsWhileOnARouter};
use resilum_core::ble::radio::Radio;
use resilum_core::{BleInterface, Config, Node};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("resilum-ble-vote-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_node_that_can_host(tag: &str) -> Node {
    let mut node = Node::new(Config {
        storage_path: Some(temp_dir(tag)),
        discover_interfaces: false,
        ble: Some(BleInterface {
            can_host_a_group: true,
        }),
        ..Config::minimal(format!("ble-vote-{tag}-{}", std::process::id()))
    })
    .expect("node");
    node.start().expect("start");
    node
}

fn on_a_router_with_an_uplink() -> Facts {
    Facts {
        has_an_uplink: true,
        p2p_and_sta_at_once: HostsWhileOnARouter::Confirmed,
        charging: true,
        battery_percent: 100,
        neighbours_heard: 0,
    }
}

fn waited_for_a_candidate_that_has_spoken(node: &Node) -> Option<Candidate> {
    (0..200).find_map(|_| {
        let spoken = node
            .ble_candidates()
            .into_iter()
            .find(|candidate| candidate.can_host_at_all());
        if spoken.is_none() {
            std::thread::sleep(Duration::from_millis(50));
        }
        spoken
    })
}

#[test]
fn what_a_candidate_is_reaches_the_other_candidate_over_the_mesh() {
    let air = Air::new(CARRIED_PER_WRITE);
    let mut listener = a_node_that_can_host("listener");
    let mut speaker = a_node_that_can_host("speaker");
    speaker.ble_facts_reported(on_a_router_with_an_uplink(), true);

    let listening_radio: Arc<dyn Radio> = Arc::new(FakeRadio::on(&air, "aa:vote"));
    let speaking_radio: Arc<dyn Radio> = Arc::new(FakeRadio::on(&air, "bb:vote"));
    speaker
        .ble_attach_a_radio_the_caller_owns(speaking_radio)
        .expect("the speaker takes the radio");
    listener
        .ble_attach_a_radio_the_caller_owns(listening_radio)
        .expect("the listener takes the radio");

    let heard = waited_for_a_candidate_that_has_spoken(&listener)
        .expect("the peer met over the air never said what it was");

    assert!(heard.facts().has_an_uplink);
    assert_eq!(
        heard.facts().p2p_and_sta_at_once,
        HostsWhileOnARouter::Confirmed
    );
    assert_eq!(heard.facts().neighbours_heard, 1);
}
