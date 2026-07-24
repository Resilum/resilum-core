//! End-to-end: one egress node registers two services, and each inbound link is
//! routed to its own service's target (by the destination_hash on the link).

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use leviculum_std::NodeEvent;
use leviculum_std::api::{Destination, Node as LevNode};
use resilum_core::dispatch::Events;
use resilum_core::{Config, EgressListen, Node};
use tokio::time::timeout;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("resilum-ms-{tag}-{}", std::process::id()))
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Echo server that prefixes every reply with `tag`, so callers can tell which
/// target they reached.
fn spawn_tagged_echo(tag: u8) -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut stream = stream;
                let mut buf = [0u8; 1024];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let mut reply = vec![tag];
                    reply.extend_from_slice(&buf[..n]);
                    if stream.write_all(&reply).is_err() {
                        break;
                    }
                }
            });
        }
    });
    port
}

fn start_egress(services: Vec<EgressListen>, dir: &std::path::Path) -> (Node, u16) {
    (0..10)
        .find_map(|_| {
            let port = free_port();
            let cfg = Config {
                storage_path: Some(dir.to_path_buf()),
                discover_interfaces: false,
                listen: Some(format!("127.0.0.1:{port}")),
                egress: services.clone(),
                ..Config::minimal(format!("ms-egress-{}", std::process::id()))
            };
            let mut node = Node::new(cfg).expect("new");
            node.start().is_ok().then_some((node, port))
        })
        .expect("egress node started")
}

/// Connect to `service`'s destination, send a byte, and return the first byte of
/// the reply (the target's tag).
async fn probe_service(engine: &Arc<LevNode>, events: &Events, service: &str) -> u8 {
    let mut ev = events.subscribe();
    let want = Destination::compute_name_hash("resilum", &["bridge", "tcp", service]);

    let (dest_hash, key) = timeout(Duration::from_secs(20), async {
        loop {
            if let Ok(event) = ev.recv().await
                && let NodeEvent::AnnounceReceived { announce, .. } = &*event
                && announce.name_hash() == &want
            {
                let key: [u8; 32] = announce.public_key()[32..64].try_into().unwrap();
                break (*announce.destination_hash(), key);
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("discover {service}"));

    let handle = engine
        .connect_with_key(&dest_hash, &key)
        .await
        .expect("connect");
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
    .expect("established");

    handle.send(b"ping").await.expect("send");

    timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(event) = ev.recv().await
                && let NodeEvent::MessageReceived {
                    link_id: id, data, ..
                } = &*event
                && *id == link_id
            {
                break data[0];
            }
        }
    })
    .await
    .expect("reply")
}

#[test]
fn each_service_is_routed_to_its_own_target() {
    let echo_a = spawn_tagged_echo(b'A');
    let echo_b = spawn_tagged_echo(b'B');

    let dir = temp_dir("egress");
    let mut svc_a = EgressListen::new("svc-a", Some(format!("127.0.0.1:{echo_a}")));
    svc_a.announce_interval = Duration::from_secs(1);
    let mut svc_b = EgressListen::new("svc-b", Some(format!("127.0.0.1:{echo_b}")));
    svc_b.announce_interval = Duration::from_secs(1);
    let (mut egress, egress_port) = start_egress(vec![svc_a, svc_b], &dir);

    let dir_c = temp_dir("client");
    let mut client = Node::new(Config {
        storage_path: Some(dir_c.clone()),
        discover_interfaces: false,
        bootstrap: vec![format!("127.0.0.1:{egress_port}")],
        ..Config::minimal(format!("ms-client-{}", std::process::id()))
    })
    .expect("new");
    client.start().expect("client start");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let engine = client.engine().expect("engine");
    let events = client.events().clone();
    let (a, b) = rt.block_on(async {
        let a = probe_service(&engine, &events, "svc-a").await;
        let b = probe_service(&engine, &events, "svc-b").await;
        (a, b)
    });

    client.stop().ok();
    egress.stop().ok();
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(dir_c);

    assert_eq!(a, b'A', "svc-a reached the A target");
    assert_eq!(b, b'B', "svc-b reached the B target");
}
