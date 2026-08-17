use std::time::Duration;

use crate::upstream::{Deadlines, Upstream};

/// The server here is a socket that reads as open for ever: it completes the
/// handshake and then never speaks, never answers a ping and never closes.
#[tokio::test]
async fn a_relay_that_stops_speaking_is_dropped_and_redialled() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (seen_tx, mut seen_rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        // Held, never polled: an unpolled tungstenite stream answers no ping
        // and sends no close, while dropping it would close the socket and
        // give the client the drop it is not supposed to get here.
        let mut held = Vec::new();
        for round in 0..2 {
            let (stream, _) = listener.accept().await.expect("accept");
            held.push(
                tokio_tungstenite::accept_async(stream)
                    .await
                    .expect("handshake"),
            );
            let _ = seen_tx.send(round);
        }
        std::future::pending::<()>().await;
    });

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let (_handle, runner) = Upstream::pair(format!("ws://127.0.0.1:{port}"), tx);
    let runner = runner.with_deadlines(Deadlines {
        idle: Duration::from_millis(100),
        answer: Duration::from_millis(100),
    });
    tokio::spawn(runner.run(Vec::new));

    let rounds = tokio::time::timeout(Duration::from_secs(10), async {
        (seen_rx.recv().await, seen_rx.recv().await)
    })
    .await
    .expect("the silent relay was never dropped, so it was never re-dialled");

    assert_eq!(rounds, (Some(0), Some(1)));
}

/// A relay says nothing at all between events, so silence is the normal
/// state of a healthy connection and must not be what ends it. This is the
/// other half of the deadline: only silence that survives a ping counts.
#[tokio::test]
async fn a_quiet_relay_that_answers_its_pings_keeps_the_same_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (ping_tx, mut ping_rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut round = 0;
        loop {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(stream)
                .await
                .expect("handshake");
            // Reading is what makes tungstenite answer a ping with a pong.
            while let Some(Ok(message)) = futures_util::StreamExt::next(&mut ws).await {
                if message.is_ping() {
                    let _ = ping_tx.send(round);
                }
            }
            round += 1;
        }
    });

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let (handle, runner) = Upstream::pair(format!("ws://127.0.0.1:{port}"), tx);
    let runner = runner.with_deadlines(Deadlines {
        idle: Duration::from_millis(50),
        answer: Duration::from_millis(50),
    });
    tokio::spawn(runner.run(Vec::new));

    let rounds = tokio::time::timeout(Duration::from_secs(10), async {
        let mut rounds = Vec::new();
        for _ in 0..4 {
            rounds.push(ping_rx.recv().await);
        }
        rounds
    })
    .await
    .expect("the quiet relay stopped being pinged");

    assert_eq!(rounds, vec![Some(0); 4], "the connection was re-dialled");
    assert!(handle.is_up(), "the connection was given up on");
}
