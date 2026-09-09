/// Left set, `is_up` has `send` filling an outbox nobody will ever drain.
#[tokio::test]
async fn is_up_clears_even_when_run_is_cancelled_mid_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let _ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("handshake");
        std::future::pending::<()>().await; // hold the connection open
    });

    let (handle, runner, _arriving) = super::a_relay_on(port);
    let task = tokio::spawn(runner.run(Vec::new));

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !handle.is_up() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("connection never came up");

    task.abort();

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while handle.is_up() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("is_up stayed true after the runner was aborted");
}
