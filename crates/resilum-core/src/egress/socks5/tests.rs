use super::*;
use crate::link::LinkMsg;
use tokio::sync::mpsc::{self, UnboundedReceiver};

fn feed(bytes: &[u8]) -> UnboundedReceiver<LinkMsg> {
    let (tx, rx) = mpsc::unbounded_channel();
    tx.send(LinkMsg::Data(bytes.to_vec())).unwrap();
    drop(tx);
    rx
}

#[tokio::test]
async fn parses_domain_connect() {
    let mut rx = feed(&[
        0x05, 0x01, 0x00, 0x05, 0x01, 0x00, 0x03, 11, b'e', b'x', b'a', b'm', b'p', b'l', b'e',
        b'.', b'c', b'o', b'm', 0x00, 0x50,
    ]);
    let req = handshake(&mut rx).await.unwrap();
    assert_eq!(req.host, "example.com");
    assert_eq!(req.port, 80);
    assert!(req.leftover.is_empty());
}

#[tokio::test]
async fn parses_ipv4_connect_with_leftover() {
    let mut rx = feed(&[
        0x05, 0x01, 0x00, 0x05, 0x01, 0x00, 0x01, 198, 18, 0, 1, 0x01, 0xbb, b'G', b'E', b'T',
    ]);
    let req = handshake(&mut rx).await.unwrap();
    assert_eq!(req.host, "198.18.0.1");
    assert_eq!(req.port, 443);
    assert_eq!(req.leftover, b"GET");
}

#[tokio::test]
async fn rejects_bind_command() {
    let mut rx = feed(&[0x05, 0x01, 0x00, 0x05, 0x02, 0x00, 0x01, 1, 1, 1, 1, 0, 80]);
    assert!(matches!(
        handshake(&mut rx).await,
        Err(SocksError::UnsupportedCommand(0x02))
    ));
}
