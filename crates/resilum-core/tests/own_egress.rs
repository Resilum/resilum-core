//! A node that runs an exit itself reaches it over a local socket, with no
//! peer and no mesh round trip.

use std::io::{Read, Write};
use std::time::Duration;

use resilum_core::{Config, EgressListen, IngressConfig, Node};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a local port")
        .local_addr()
        .expect("the port just bound")
        .port()
}

fn spawn_echo() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a local port");
    let port = listener.local_addr().expect("the port just bound").port();
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

#[test]
fn a_node_uses_the_exit_it_runs_itself() {
    let echo = spawn_echo();
    let listen_tcp = free_port();
    let dir = std::env::temp_dir().join(format!("resilum-own-{}", std::process::id()));

    let mut node = Node::new(Config {
        storage_path: Some(dir.clone()),
        discover_interfaces: false,
        egress: vec![EgressListen::new("e2e", Some(format!("127.0.0.1:{echo}")))],
        ingress: Some(IngressConfig::new("e2e", format!("127.0.0.1:{listen_tcp}"))),
        ..Config::minimal(format!("own-egress-{}", std::process::id()))
    })
    .expect("new");
    node.start().expect("start");

    assert_eq!(
        node.registry().all().len(),
        1,
        "the exit this node runs is a candidate for its own traffic"
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
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read deadline");
    stream.write_all(b"ping").expect("to send");

    let mut buf = [0u8; 4];
    let got = stream.read_exact(&mut buf);

    node.stop().ok();
    let _ = std::fs::remove_dir_all(dir);

    got.expect("a round trip through this node's own exit");
    assert_eq!(&buf, b"ping");
}
