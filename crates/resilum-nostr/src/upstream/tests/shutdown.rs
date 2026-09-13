use futures_util::SinkExt as _;
use tokio_tungstenite::tungstenite::Message;

/// `run` owns its own clone of the shared state, so nothing else stops it:
/// this receiver is the caller's only lever.
#[tokio::test]
async fn dropping_the_incoming_receiver_ends_the_reconnect_loop() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();

    resilum_tasks::watch("a test relay that stays open", async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("handshake");
        ws.send(Message::text(r#"["NOTICE","still here"]"#))
            .await
            .expect("to greet the client");
        std::future::pending::<()>().await; // keep the socket open; `run` must exit on its own
    });

    let (_handle, runner, arriving) = super::a_relay_on(port);
    drop(arriving);
    let task = resilum_tasks::watch("the upstream under test", runner.run(Vec::new));

    tokio::time::timeout(std::time::Duration::from_secs(10), task.come_home())
        .await
        .expect("run did not exit after its incoming receiver was dropped");
}
