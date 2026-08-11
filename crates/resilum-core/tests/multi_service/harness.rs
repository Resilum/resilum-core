//! The two ends the test needs: a tagged echo target, and an egress node.

use std::io::{Read, Write};
use std::time::Duration;

use resilum_core::{Config, EgressListen, Node};

use crate::support::free_port;

/// Echo server that prefixes every reply with `tag`, so callers can tell which
/// target they reached.
pub fn spawn_tagged_echo(tag: u8) -> u16 {
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

/// A service pair announced often enough for the test not to wait on it.
pub fn services(echo_a: u16, echo_b: u16) -> Vec<EgressListen> {
    [("svc-a", echo_a), ("svc-b", echo_b)]
        .into_iter()
        .map(|(name, echo)| {
            let mut svc = EgressListen::new(name, Some(format!("127.0.0.1:{echo}")));
            svc.announce_interval = Duration::from_secs(1);
            svc
        })
        .collect()
}

/// Retries because the listen port is chosen before it is bound.
pub fn start_egress(services: Vec<EgressListen>, dir: &std::path::Path) -> (Node, u16) {
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

pub fn start_client(dir: &std::path::Path, egress_port: u16) -> Node {
    let mut client = Node::new(Config {
        storage_path: Some(dir.to_path_buf()),
        discover_interfaces: false,
        bootstrap: vec![format!("127.0.0.1:{egress_port}")],
        ..Config::minimal(format!("ms-client-{}", std::process::id()))
    })
    .expect("new");
    client.start().expect("client start");
    client
}
