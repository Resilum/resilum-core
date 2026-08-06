//! Bridges one iroh QUIC bidi stream onto a leviculum byte-channel, so an RNS
//! link rides the connection exactly as it would over TCP.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use iroh::EndpointId;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use leviculum_std::api::Node as LevNode;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::Links;

/// A QUIC bidi stream as one duplex: reads pull from the recv half, writes push
/// to the send half.
pub struct IrohStream {
    send: SendStream,
    recv: RecvStream,
}

impl AsyncRead for IrohStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.recv).poll_read(cx, buf)
    }
}

impl AsyncWrite for IrohStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // The inherent quinn `poll_write` (WriteError) shadows the tokio trait
        // method, so call the trait method explicitly.
        AsyncWrite::poll_write(Pin::new(&mut self.send), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.send).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.send).poll_shutdown(cx)
    }
}

/// Accept the peer's first bidi stream on an inbound connection and bridge it.
pub async fn accept_link(engine: &LevNode, links: &Links, conn: Connection) {
    match conn.accept_bi().await {
        Ok((send, recv)) => register(engine, links, conn.remote_id(), send, recv),
        Err(e) => tracing::warn!(error = %e, "iroh accept_bi failed"),
    }
}

/// Open a bidi stream on an outbound connection and bridge it.
pub async fn dial_link(engine: &LevNode, links: &Links, conn: Connection) {
    match conn.open_bi().await {
        Ok((send, recv)) => register(engine, links, conn.remote_id(), send, recv),
        Err(e) => tracing::warn!(error = %e, "iroh open_bi failed"),
    }
}

fn register(engine: &LevNode, links: &Links, id: EndpointId, send: SendStream, recv: RecvStream) {
    let name = format!("iroh[{}]", id.fmt_short());
    match engine.spawn_byte_channel(&name, IrohStream { send, recv }) {
        Ok(handle) => {
            tracing::info!(%name, "attached RNS peer over iroh");
            // Inserting drops any prior handle for this peer, detaching a stale
            // link it is re-dialing over.
            links.lock().expect("iroh links").insert(id, handle);
        }
        Err(e) => tracing::warn!(%name, error = %e, "iroh byte-channel attach failed"),
    }
}
