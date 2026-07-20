//! Bidirectional byte pump between a TCP stream and one link session.
//! Either side ending tears down the pump.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::link::LinkMsg;

const CHUNK: usize = 8 * 1024;

/// `from_link` carries inbound link bytes for the TCP peer; `to_link` collects
/// TCP bytes for the link driver to send. Returns when either side ends.
pub async fn pump<S>(
    mut tcp: S,
    mut from_link: UnboundedReceiver<LinkMsg>,
    to_link: UnboundedSender<Vec<u8>>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; CHUNK];
    loop {
        tokio::select! {
            msg = from_link.recv() => match msg {
                Some(LinkMsg::Data(bytes)) => {
                    if tcp.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Some(LinkMsg::Established) => {}
                Some(LinkMsg::Closed) | None => break,
            },
            read = tcp.read(&mut buf) => match read {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if to_link.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn link_data_reaches_the_tcp_peer() {
        let (side, mut peer) = tokio::io::duplex(256);
        let (ltx, lrx) = mpsc::unbounded_channel();
        let (otx, _orx) = mpsc::unbounded_channel();
        let task = tokio::spawn(pump(side, lrx, otx));

        ltx.send(LinkMsg::Data(b"hello".to_vec())).unwrap();
        let mut got = [0u8; 5];
        peer.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"hello");

        drop(ltx);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn tcp_bytes_reach_the_link_channel() {
        let (side, mut peer) = tokio::io::duplex(256);
        let (_ltx, lrx) = mpsc::unbounded_channel();
        let (otx, mut orx) = mpsc::unbounded_channel();
        let task = tokio::spawn(pump(side, lrx, otx));

        peer.write_all(b"world").await.unwrap();
        assert_eq!(orx.recv().await.unwrap(), b"world".to_vec());

        drop(peer);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn link_close_ends_the_pump() {
        let (side, _peer) = tokio::io::duplex(256);
        let (ltx, lrx) = mpsc::unbounded_channel();
        let (otx, _orx) = mpsc::unbounded_channel();
        let task = tokio::spawn(pump(side, lrx, otx));

        ltx.send(LinkMsg::Closed).unwrap();
        task.await.unwrap();
    }
}
