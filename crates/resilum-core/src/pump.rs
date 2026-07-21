//! Bidirectional byte pump between a TCP stream and one link session.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::link::LinkMsg;

const CHUNK: usize = 8 * 1024;

pub async fn pump<S>(
    mut tcp: S,
    mut from_link: UnboundedReceiver<LinkMsg>,
    to_link: UnboundedSender<Vec<u8>>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; CHUNK];
    let mut tcp_read_done = false;
    loop {
        tokio::select! {
            msg = from_link.recv() => match msg {
                Some(LinkMsg::Data(bytes)) => {
                    if tcp.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Some(LinkMsg::Established) => {}
                Some(LinkMsg::Closed) | None => {
                    let _ = tcp.shutdown().await;
                    break;
                }
            },
            // TCP EOF stops only this direction; link→TCP keeps flowing for the reply.
            read = tcp.read(&mut buf), if !tcp_read_done => match read {
                Ok(0) | Err(_) => tcp_read_done = true,
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
        let (ltx, lrx) = mpsc::unbounded_channel();
        let (otx, mut orx) = mpsc::unbounded_channel();
        let task = tokio::spawn(pump(side, lrx, otx));

        peer.write_all(b"world").await.unwrap();
        assert_eq!(orx.recv().await.unwrap(), b"world".to_vec());

        drop(peer);
        drop(ltx);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn reply_flows_after_the_client_half_closes() {
        let (side, mut peer) = tokio::io::duplex(256);
        let (ltx, lrx) = mpsc::unbounded_channel();
        let (otx, _orx) = mpsc::unbounded_channel();
        let task = tokio::spawn(pump(side, lrx, otx));

        peer.shutdown().await.unwrap(); // client stops sending (our TCP read hits EOF)
        ltx.send(LinkMsg::Data(b"resp".to_vec())).unwrap();
        let mut got = [0u8; 4];
        peer.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"resp");

        ltx.send(LinkMsg::Closed).unwrap();
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
