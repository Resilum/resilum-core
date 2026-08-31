#[path = "support/fake_radio/mod.rs"]
mod fake_radio;

use std::sync::Arc;
use std::time::Duration;

use fake_radio::{Air, CARRIED_PER_WRITE, FakeRadio};
use resilum_core::ble::radio::Radio;
use resilum_core::{BleInterface, Config, Node};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("resilum-ble-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_node(tag: &str) -> Node {
    let mut node = Node::new(Config {
        storage_path: Some(temp_dir(tag)),
        discover_interfaces: false,
        ble: Some(BleInterface::default()),
        ..Config::minimal(format!("ble-{tag}-{}", std::process::id()))
    })
    .expect("node");
    node.start().expect("start");
    node
}

fn interfaces_named_ble(node: &Node) -> usize {
    let Some(engine) = node.engine() else {
        return 0;
    };
    engine
        .interface_stats()
        .iter()
        .filter(|i| i.name.starts_with("BleDiscovered["))
        .count()
}

fn waited_for(node: &Node, how_many: usize) -> bool {
    (0..300).any(|_| {
        if interfaces_named_ble(node) >= how_many {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
        false
    })
}

#[test]
fn two_nodes_meeting_over_the_air_each_gain_the_other_as_an_interface() {
    let air = Air::new(CARRIED_PER_WRITE);
    let mut dialer = a_node("dialer");
    let mut answerer = a_node("answerer");

    let dialing_radio: Arc<dyn Radio> = Arc::new(FakeRadio::on(&air, "aa:ee"));
    let answering_radio: Arc<dyn Radio> = Arc::new(FakeRadio::on(&air, "bb:ee"));

    answerer
        .ble_attach_a_radio_the_caller_owns(answering_radio)
        .expect("answerer takes the radio");
    dialer
        .ble_attach_a_radio_the_caller_owns(dialing_radio)
        .expect("dialer takes the radio");

    assert!(
        waited_for(&dialer, 1),
        "the dialer never attached the peer it met"
    );
    assert!(
        waited_for(&answerer, 1),
        "the answerer never attached the peer that met it"
    );

    dialer.stop().expect("stop");
    answerer.stop().expect("stop");
}
