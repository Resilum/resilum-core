//! End-to-end: a connect node accepts local TCP and forwards it through the
//! mesh to an egress node, which relays to a local TCP echo and back.

use std::io::{Read, Write};
use std::time::Duration;

use resilum_core::{Config, ConnectConfig, EgressListen, Node};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("resilum-ce-{tag}-{}", std::process::id()))
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

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

fn start_egress(echo: u16) -> (Node, u16) {
    let dir = temp_dir("egress");
    (0..10)
        .find_map(|_| {
            let port = free_port();
            let mut egress = EgressListen::new("e2e", format!("127.0.0.1:{echo}"));
            egress.announce_interval = Duration::from_secs(1);
            let cfg = Config {
                storage_path: Some(dir.clone()),
                discover_interfaces: false,
                listen: Some(format!("127.0.0.1:{port}")),
                egress: vec![egress],
                ..Config::minimal(format!("ce-egress-{}", std::process::id()))
            };
            let mut node = Node::new(cfg).expect("new");
            node.start().is_ok().then_some((node, port))
        })
        .expect("egress node started")
}

#[test]
fn connect_forwards_a_local_connection_through_egress() {
    let echo = spawn_echo();
    let (mut egress, egress_port) = start_egress(echo);

    let listen_tcp = free_port();
    let dir_c = temp_dir("connect");
    let mut connect = Node::new(Config {
        storage_path: Some(dir_c.clone()),
        discover_interfaces: false,
        bootstrap: vec![format!("127.0.0.1:{egress_port}")],
        connect: Some(ConnectConfig::new("e2e", format!("127.0.0.1:{listen_tcp}"))),
        ..Config::minimal(format!("ce-connect-{}", std::process::id()))
    })
    .expect("new");
    connect.start().expect("connect start");

    let discovered = (0..200).any(|_| {
        if !connect.registry().all().is_empty() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
        false
    });
    assert!(discovered, "egress candidate was discovered");
    assert_eq!(
        connect.socks_port(),
        listen_tcp,
        "bound SOCKS port is reported"
    );

    let mut stream = (0..40)
        .find_map(|_| {
            std::net::TcpStream::connect(("127.0.0.1", listen_tcp))
                .ok()
                .or_else(|| {
                    std::thread::sleep(Duration::from_millis(50));
                    None
                })
        })
        .expect("connect to the local listener");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    stream.write_all(b"ping").unwrap();

    let mut buf = [0u8; 4];
    let got = stream.read_exact(&mut buf);

    // discovery surfaced the egress peer, with its destination hash
    let mut discovered_hash = None;
    while let Some(event) = connect.poll_event() {
        if let resilum_core::Event::PeerDiscovered(hash) = event {
            discovered_hash = Some(hash);
        }
    }

    connect.stop().ok();
    egress.stop().ok();
    let _ = std::fs::remove_dir_all(temp_dir("egress"));
    let _ = std::fs::remove_dir_all(dir_c);

    got.expect("round-trip through the mesh");
    assert_eq!(&buf, b"ping");
    assert!(
        discovered_hash.is_some_and(|h| !h.is_empty()),
        "PeerDiscovered event carried a hash"
    );
}
