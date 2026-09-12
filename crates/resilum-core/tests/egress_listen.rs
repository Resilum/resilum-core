//! End-to-end: an egress listen node accepts an inbound link and forwards its
//! bytes to a local TCP endpoint, echoing them back to the initiator.

use std::io::{Read, Write};
use std::time::Duration;

use leviculum_std::NodeEvent;
use leviculum_std::api::Destination;
use resilum_core::{Config, EgressListen, Node};
use tokio::time::timeout;

mod support;

use support::free_port;

use support::temp_dir;

/// A blocking TCP echo server on an ephemeral port.
fn spawn_echo() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut stream = stream;
                let mut buf = [0u8; 1024];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 || stream.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            });
        }
    });
    port
}

fn start_egress(echo: u16) -> (Node, u16, tempfile::TempDir) {
    let dir = temp_dir();
    let (node, port) = (0..10)
        .find_map(|_| {
            let port = free_port();
            let mut egress = EgressListen::new("e2e", Some(format!("127.0.0.1:{echo}")));
            egress.announce_interval = Duration::from_secs(1);
            let cfg = Config {
                storage_path: Some(dir.path().to_path_buf()),
                discover_interfaces: false,
                listen: Some(format!("127.0.0.1:{port}")),
                egress: vec![egress],
                ..Config::minimal(format!("egr-listen-{}", std::process::id()))
            };
            let mut node = Node::new(cfg).expect("new");
            node.start().is_ok().then_some((node, port))
        })
        .expect("egress node started");
    (node, port, dir)
}

#[test]
fn egress_forwards_link_bytes_to_local_tcp() {
    let echo = spawn_echo();
    let (mut egress, egress_port, _egress_dir) = start_egress(echo);

    let dir_b = temp_dir();
    let mut client = Node::new(Config {
        storage_path: Some(dir_b.path().to_path_buf()),
        discover_interfaces: false,
        bootstrap: vec![format!("127.0.0.1:{egress_port}")],
        ..Config::minimal(format!("egr-conn-{}", std::process::id()))
    })
    .expect("new");
    client.start().expect("client start");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let echoed = rt.block_on(async {
        let engine = client.engine().expect("engine");
        let mut ev = client.events().subscribe();
        let want_name = Destination::compute_name_hash("resilum", &["bridge", "tcp", "e2e"]);

        let (dest_hash, key) = timeout(Duration::from_secs(20), async {
            loop {
                if let Ok(event) = ev.recv().await
                    && let NodeEvent::AnnounceReceived { announce, .. } = &*event
                    && announce.name_hash() == &want_name
                {
                    let key: [u8; 32] = announce.public_key()[32..64].try_into().unwrap();
                    break (*announce.destination_hash(), key);
                }
            }
        })
        .await
        .expect("discovered the egress announce");

        let handle = engine.connect(&dest_hash, &key).await.expect("connect");
        let link_id = *handle.link_id();

        timeout(Duration::from_secs(10), async {
            loop {
                if let Ok(event) = ev.recv().await
                    && let NodeEvent::LinkEstablished { link_id: id, .. } = &*event
                    && *id == link_id
                {
                    break;
                }
            }
        })
        .await
        .expect("link established");

        handle.send(b"ping").await.expect("send");

        timeout(Duration::from_secs(10), async {
            loop {
                if let Ok(event) = ev.recv().await
                    && let NodeEvent::MessageReceived {
                        link_id: id, data, ..
                    } = &*event
                    && *id == link_id
                {
                    break data.clone();
                }
            }
        })
        .await
        .expect("echo")
    });

    client.stop().ok();
    egress.stop().ok();

    assert_eq!(echoed, b"ping");
}
