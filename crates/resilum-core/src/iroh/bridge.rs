//! Bridges one iroh QUIC bidi stream onto a leviculum byte-channel, so an RNS
//! link rides the connection exactly as it would over TCP.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use iroh::EndpointId;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::wiring::{DetachesBothHalves, NOT_NAMED_UNTIL_THEY_ANNOUNCE, Wiring, attached_as};
use crate::coordinates::PeerId;
use crate::discovery::attachments::Attached;

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
pub async fn accept_link(wiring: &Wiring, conn: Connection) {
    match conn.accept_bi().await {
        Ok((send, recv)) => register(
            wiring,
            conn.remote_id(),
            NOT_NAMED_UNTIL_THEY_ANNOUNCE,
            send,
            recv,
        ),
        Err(e) => tracing::warn!(error = %e, "iroh accept_bi failed"),
    }
}

/// Open a bidi stream on an outbound connection and bridge it.
pub async fn dial_link(wiring: &Wiring, conn: Connection, announced_by: Option<PeerId>) {
    match conn.open_bi().await {
        Ok((send, recv)) => register(wiring, conn.remote_id(), announced_by, send, recv),
        Err(e) => tracing::warn!(error = %e, "iroh open_bi failed"),
    }
}

fn register(
    wiring: &Wiring,
    id: EndpointId,
    announced_by: Option<PeerId>,
    send: SendStream,
    recv: RecvStream,
) {
    let name = attached_as(id);
    match wiring
        .engine
        .spawn_byte_channel(&name, IrohStream { send, recv })
    {
        Ok(handle) => {
            let interface = handle.id();
            wiring.origin.record(interface, super::ORIGIN);
            tracing::info!(%name, "attached RNS peer over iroh");
            wiring.attachments.hold(
                name,
                Attached {
                    service: super::ORIGIN.to_owned(),
                    announced_by,
                    interface,
                    _detaches_when_dropped: Box::new(DetachesBothHalves::new(
                        wiring.engine.clone(),
                        handle,
                    )),
                },
            );
        }
        Err(e) => tracing::warn!(%name, error = %e, "iroh byte-channel attach failed"),
    }
}
