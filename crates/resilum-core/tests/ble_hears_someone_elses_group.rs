#[path = "support/fake_radio/mod.rs"]
mod fake_radio;

use std::sync::Arc;
use std::time::Duration;

use fake_radio::{Air, CARRIED_PER_WRITE, FakeRadio};
use resilum_core::ble::beacon::Beacon;
use resilum_core::ble::radio::Radio;
use resilum_core::ble::spec;
use resilum_core::{BleInterface, Config, Node};

fn a_node_looking_around(tag: &str) -> Node {
    let dir = std::env::temp_dir().join(format!("resilum-ble-air-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut node = Node::new(Config {
        storage_path: Some(dir),
        discover_interfaces: false,
        ble: Some(BleInterface {
            can_host_a_group: true,
        }),
        ..Config::minimal(format!("ble-air-{tag}-{}", std::process::id()))
    })
    .expect("node");
    node.start().expect("start");
    node
}

fn within_a_few_look_arounds(node: &Node, wanted: bool) -> bool {
    (0..300)
        .find(|_| {
            if node.ble_someone_else_hosts_a_group() == wanted {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
            false
        })
        .is_some()
}

#[test]
fn a_neighbour_advertising_a_raised_group_is_one_we_could_join() {
    let air = Air::new(CARRIED_PER_WRITE);
    let mut node = a_node_looking_around("hears");
    let ours: Arc<dyn Radio> = Arc::new(FakeRadio::on(&air, "aa:air"));
    node.ble_attach_a_radio_the_caller_owns(ours)
        .expect("the node takes the radio");
    let neighbour = FakeRadio::on(&air, "bb:air");

    neighbour
        .advertise("", &Beacon::fresh(true, true).on_the_air(), spec::SERVICE)
        .expect("the neighbour goes on the air");

    assert!(
        within_a_few_look_arounds(&node, true),
        "a group raised in earshot never reached the node"
    );
}

#[test]
fn a_neighbour_still_announcing_in_its_name_is_heard_all_the_same() {
    let air = Air::new(CARRIED_PER_WRITE);
    let mut node = a_node_looking_around("named");
    let ours: Arc<dyn Radio> = Arc::new(FakeRadio::on(&air, "ee:air"));
    node.ble_attach_a_radio_the_caller_owns(ours)
        .expect("the node takes the radio");
    let neighbour = FakeRadio::on(&air, "ff:air");

    neighbour
        .advertise(&Beacon::fresh(true, true).name(), &[], spec::SERVICE)
        .expect("the neighbour goes on the air");

    assert!(
        within_a_few_look_arounds(&node, true),
        "a group announced the old way never reached the node"
    );
}

#[test]
fn a_neighbour_that_raised_nothing_is_not_a_group_to_join() {
    let air = Air::new(CARRIED_PER_WRITE);
    let mut node = a_node_looking_around("quiet");
    let ours: Arc<dyn Radio> = Arc::new(FakeRadio::on(&air, "cc:air"));
    node.ble_attach_a_radio_the_caller_owns(ours)
        .expect("the node takes the radio");
    let neighbour = FakeRadio::on(&air, "dd:air");

    neighbour
        .advertise("", &Beacon::fresh(true, false).on_the_air(), spec::SERVICE)
        .expect("the neighbour goes on the air");

    std::thread::sleep(Duration::from_secs(1));

    assert!(
        !node.ble_someone_else_hosts_a_group(),
        "a neighbour that hosts nothing was read as a group"
    );
}
