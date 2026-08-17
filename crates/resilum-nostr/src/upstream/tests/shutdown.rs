use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

use crate::upstream::Upstream;

/// `run` owns its own clone of the shared state, so nothing else stops it;
/// the caller's only lever is dropping the `incoming` receiver. A relay
/// that keeps talking after that must not keep the task alive forever,
/// parsing and discarding frames nobody can read.
#[tokio::test]
async fn dropping_the_incoming_receiver_ends_the_reconnect_loop() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("handshake");
        let _ = ws.send(Message::text(r#"["NOTICE","still here"]"#)).await;
        std::future::pending::<()>().await; // keep the socket open; `run` must exit on its own
    });

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    drop(rx);
    let (_handle, runner) = Upstream::pair(format!("ws://127.0.0.1:{port}"), tx);
    let task = tokio::spawn(runner.run(Vec::new));

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), task)
        .await
        .expect("run did not exit after its incoming receiver was dropped");
    outcome.expect("run task panicked instead of returning");
}
