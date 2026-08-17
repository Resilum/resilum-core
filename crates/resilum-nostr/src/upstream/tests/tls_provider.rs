use tokio::io::AsyncReadExt;

use crate::upstream::Upstream;

/// `wss://` builds a `rustls::ClientConfig` as soon as the TCP handshake
/// completes, before any TLS bytes are exchanged. The proof is a byte: a
/// ClientHello can only be written by a dial that got past it. The server
/// never speaks TLS, so the handshake itself is expected to fail; what must
/// not happen is a panic on the way there.
///
/// `run` is polled here rather than spawned, so a panic inside it surfaces
/// as this test's own failure instead of a silently abandoned task.
#[tokio::test]
async fn a_wss_dial_writes_tls_bytes_instead_of_panicking_for_want_of_a_provider() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (hello_tx, mut hello_rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut first = [0u8; 1];
        let read = stream.read(&mut first).await.unwrap_or(0);
        let _ = hello_tx.send(read);
    });

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let (_handle, runner) = Upstream::pair(format!("wss://127.0.0.1:{port}"), tx);

    let read = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::select! {
            () = runner.run(Vec::new) => unreachable!("the reconnect loop never returns"),
            read = hello_rx.recv() => read.expect("the server task ended without reporting"),
        }
    })
    .await
    .expect("the wss dial never reached the server");

    assert_eq!(
        read, 1,
        "the dial closed the socket instead of starting TLS"
    );
}
