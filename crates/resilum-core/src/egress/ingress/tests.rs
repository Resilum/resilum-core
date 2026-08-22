use std::net::Ipv4Addr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

const GREETING_NO_AUTH: [u8; 3] = [5, 1, 0];
const CONNECT_TO_AN_ADDRESS: [u8; 10] = [5, 1, 0, 1, 203, 0, 113, 1, 0, 80];

#[tokio::test]
async fn a_connection_with_nowhere_to_go_is_told_why_rather_than_dropped() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("a local port to listen on");
    let listening_on = listener.local_addr().expect("the port just bound");

    tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("the client below");
        turn_away(tcp, REP_NO_EGRESS_TO_REACH_THE_INTERNET_THROUGH).await;
    });

    let mut client = TcpStream::connect(listening_on)
        .await
        .expect("the listener above");
    client
        .write_all(&GREETING_NO_AUTH)
        .await
        .expect("to greet the proxy");
    let mut chosen_method = [0u8; 2];
    client
        .read_exact(&mut chosen_method)
        .await
        .expect("an answer to the greeting");
    assert_eq!(chosen_method, [5, 0]);

    client
        .write_all(&CONNECT_TO_AN_ADDRESS)
        .await
        .expect("to ask for a connection");
    let mut answer = [0u8; 10];
    client
        .read_exact(&mut answer)
        .await
        .expect("a refusal a client can read");

    assert_eq!(answer[1], REP_NO_EGRESS_TO_REACH_THE_INTERNET_THROUGH);
}
