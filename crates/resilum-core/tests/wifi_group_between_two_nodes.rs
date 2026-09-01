use std::net::{TcpListener, TcpStream};
use std::os::fd::IntoRawFd;
use std::time::Duration;

use resilum_core::{Config, Node};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("resilum-group-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_node(tag: &str) -> Node {
    let mut node = Node::new(Config {
        storage_path: Some(temp_dir(tag)),
        discover_interfaces: false,
        ..Config::minimal(format!("group-{tag}-{}", std::process::id()))
    })
    .expect("node");
    node.start().expect("start");
    node
}

fn group_interfaces(node: &Node) -> usize {
    node.engine().map_or(0, |engine| {
        engine
            .interface_stats()
            .iter()
            .filter(|i| i.name.starts_with("WifiGroup["))
            .count()
    })
}

fn waited_for(node: &Node, links: usize) -> bool {
    (0..200).any(|_| {
        if group_interfaces(node) == links {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
        false
    })
}

#[test]
fn a_phone_that_joins_the_group_and_the_one_hosting_it_both_gain_a_link() {
    let mut host = a_node("host");
    let mut guest = a_node("guest");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let group_at = listener.local_addr().expect("the group named no address");
    let hosting = host
        .wifi_group_hosting(listener.into_raw_fd())
        .expect("the host takes the listening socket");
    let dialled = TcpStream::connect(group_at).expect("join the group");
    let joined = guest
        .wifi_group_joined(dialled.into_raw_fd())
        .expect("the guest takes the connected socket");

    assert!(waited_for(&guest, 1), "the guest never gained a link");
    assert!(waited_for(&host, 1), "the host never gained a link");

    joined.detach();

    assert!(
        waited_for(&guest, 0),
        "a detached group left its link behind"
    );
    hosting.detach();

    guest.stop().expect("stop");
    host.stop().expect("stop");
}
