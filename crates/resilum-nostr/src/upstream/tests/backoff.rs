use std::time::Duration;

use crate::upstream::Upstream;

/// The first gap is checked against the documented one second from both
/// sides, not just against the second gap: a loop that doubles the wait
/// before sleeping it (the bug this guards against) still produces a growing
/// sequence, and a loop that skipped the wait entirely would too.
#[tokio::test]
async fn a_relay_that_closes_immediately_gets_backed_off_more_each_time() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (accepted_tx, mut accepted_rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(ws) = tokio_tungstenite::accept_async(stream).await else {
                return;
            };
            let _ = accepted_tx.send(std::time::Instant::now());
            drop(ws);
        }
    });

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let (_handle, runner) = Upstream::pair(format!("ws://127.0.0.1:{port}"), tx);
    let task = tokio::spawn(runner.run(Vec::new));

    let (t0, t1, t2) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        let t0 = accepted_rx.recv().await.expect("first connect");
        let t1 = accepted_rx.recv().await.expect("second connect");
        let t2 = accepted_rx.recv().await.expect("third connect");
        (t0, t1, t2)
    })
    .await
    .expect("did not see three reconnect attempts in time");
    task.abort();

    let first_gap = t1.duration_since(t0);
    let second_gap = t2.duration_since(t1);
    assert!(
        (Duration::from_secs(1)..Duration::from_millis(1_250)).contains(&first_gap),
        "the first retry must wait MIN_BACKOFF (1s), neither less nor its double: {first_gap:?}"
    );
    assert!(
        second_gap > first_gap,
        "backoff must grow after a short-lived connection: {first_gap:?} then {second_gap:?}"
    );
}
