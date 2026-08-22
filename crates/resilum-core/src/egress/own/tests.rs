use std::net::Ipv4Addr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

fn hash_of(service: &str) -> Vec<u8> {
    service.as_bytes().to_vec()
}

fn wanted() -> Vec<String> {
    vec!["tor".to_owned(), "socks-egress".to_owned()]
}

#[test]
fn an_exit_without_a_socket_to_dial_stays_reachable_only_through_the_mesh() {
    let ours = OwnExits::of(&[EgressListen::new("socks-egress", None)], hash_of);

    assert_eq!(ours.with_a_socket_this_node_can_dial(&wanted()).count(), 0);
    assert!(
        ours.is_ours(&Candidate::new(hash_of("socks-egress"), "socks-egress")),
        "it is still ours for the own/others policy"
    );
}

#[test]
fn an_exit_this_node_is_not_looking_for_is_not_offered_to_itself() {
    let configured = [EgressListen::new("i2p", Some("127.0.0.1:4447".to_owned()))];

    let ours = OwnExits::of(&configured, hash_of);

    assert_eq!(ours.with_a_socket_this_node_can_dial(&wanted()).count(), 0);
}

#[test]
fn an_exit_with_a_socket_is_found_by_the_hash_a_peer_would_dial() {
    let configured = [EgressListen::new("tor", Some("127.0.0.1:9050".to_owned()))];
    let ours = OwnExits::of(&configured, hash_of);

    assert_eq!(ours.with_a_socket_this_node_can_dial(&wanted()).count(), 1);
    assert_eq!(
        ours.target_of(&Candidate::new(hash_of("tor"), "tor")),
        Some("127.0.0.1:9050")
    );
    assert_eq!(ours.target_of(&Candidate::new(vec![9], "tor")), None);
}

#[test]
fn the_hashes_of_one_service_are_what_discovery_must_not_take_for_a_peers() {
    let configured = [
        EgressListen::new("tor", Some("127.0.0.1:9050".to_owned())),
        EgressListen::new("i2p", Some("127.0.0.1:4447".to_owned())),
    ];

    let ours = OwnExits::of(&configured, hash_of);

    assert_eq!(ours.hashes_serving("tor"), HashSet::from([hash_of("tor")]));
    assert!(ours.hashes_serving("socks-egress").is_empty());
}

#[tokio::test]
async fn a_connection_through_our_own_exit_reaches_the_local_socket() {
    let exit = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("a local port for the exit");
    let exit_address = exit.local_addr().expect("the port just bound");
    tokio::spawn(async move {
        let (mut served, _) = exit.accept().await.expect("the session below");
        let mut asked = [0u8; 3];
        served.read_exact(&mut asked).await.expect("the greeting");
        served.write_all(&[5, 0]).await.expect("to answer it");
    });

    let client_side = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("a local port for the client");
    let client_address = client_side.local_addr().expect("the port just bound");
    tokio::spawn(async move {
        let (tcp, _) = client_side.accept().await.expect("the client below");
        session(&exit_address.to_string(), tcp)
            .await
            .expect("the exit to carry it");
    });

    let mut client = tokio::net::TcpStream::connect(client_address)
        .await
        .expect("the ingress above");
    client.write_all(&[5, 1, 0]).await.expect("to greet");
    let mut answer = [0u8; 2];
    client
        .read_exact(&mut answer)
        .await
        .expect("the exit's answer to come back");

    assert_eq!(answer, [5, 0]);
}
