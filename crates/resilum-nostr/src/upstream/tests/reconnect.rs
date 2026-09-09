const REQ: &str = r#"["REQ","aabb",{"kinds":[1059]}]"#;

#[tokio::test]
async fn a_dropped_connection_comes_back_and_reissues_its_requests() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (seen_tx, mut seen_rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        for round in 0..2 {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(stream)
                .await
                .expect("handshake");
            if let Some(Ok(msg)) = futures_util::StreamExt::next(&mut ws).await {
                let _ = seen_tx.send((round, msg.to_string()));
            }
        }
    });

    let (_handle, runner, _arriving) = super::a_relay_on(port);
    tokio::spawn(runner.run(|| vec![REQ.to_owned()]));

    let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let first = seen_rx.recv().await.expect("first request");
        let second = seen_rx.recv().await.expect("request after reconnect");
        (first, second)
    })
    .await
    .expect("reconnect did not happen within 10s");

    assert_eq!(first.1, REQ);
    assert_eq!(second.1, REQ);
    // A bug that answered the first connection twice would pass the two
    // assertions above; the rounds say the second frame came from a second
    // connection.
    assert_eq!((first.0, second.0), (0, 1));
}
