//! The reconnect loop: dial, reissue subscriptions, pump frames, back off.

mod connected;
mod pump;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use futures_util::SinkExt;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::frame::Utf8Bytes;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use connected::Connected;
use pump::AfterPump;

#[cfg(test)]
pub(crate) use pump::Deadlines;

use super::{proto, tls};

const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// A connection that dropped before staying up this long has not earned a
/// reset: a relay that completes the handshake and closes immediately would
/// otherwise be retried at 1 Hz forever instead of backing off.
const BACKOFF_RESET_AFTER: Duration = Duration::from_secs(30);

/// The wait after the one just used, capped so a relay that stays down does
/// not push a caller out past a minute between attempts.
fn grow(backoff: Duration) -> Duration {
    (backoff * 2).min(MAX_BACKOFF)
}

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub struct UpstreamRunner {
    url: String,
    incoming: mpsc::UnboundedSender<proto::Incoming>,
    outbox_rx: mpsc::UnboundedReceiver<Utf8Bytes>,
    up: Arc<AtomicBool>,
    deadlines: pump::Deadlines,
}

impl UpstreamRunner {
    pub(super) fn new(
        url: String,
        incoming: mpsc::UnboundedSender<proto::Incoming>,
        outbox_rx: mpsc::UnboundedReceiver<Utf8Bytes>,
        up: Arc<AtomicBool>,
    ) -> Self {
        Self {
            url,
            incoming,
            outbox_rx,
            up,
            deadlines: pump::Deadlines::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_deadlines(mut self, deadlines: Deadlines) -> Self {
        self.deadlines = deadlines;
        self
    }

    pub async fn run(mut self, on_connect: impl Fn() -> Vec<String> + Send + 'static) {
        tls::install_default_provider();
        let mut backoff = MIN_BACKOFF;
        loop {
            match connect_async(&self.url).await {
                Ok((mut ws, _response)) => {
                    tracing::info!(url = %self.url, "connected to upstream relay");
                    // Drained here rather than before the dial: everything a
                    // sender queued while this url was down belongs to a
                    // connection that is already gone, and the window a
                    // sender can still slip a stale frame through closes when
                    // `Connected` raises the flag on the next line.
                    self.drain_outbox();
                    let up_since = Instant::now();
                    let _connected = Connected::raise(Arc::clone(&self.up));

                    let after = if self.reissue_all(&mut ws, &on_connect).await {
                        pump::pump(
                            &self.url,
                            &mut ws,
                            &self.incoming,
                            &mut self.outbox_rx,
                            &self.deadlines,
                        )
                        .await
                    } else {
                        tracing::warn!(url = %self.url, "failed to reissue subscriptions after connect");
                        AfterPump::Redial
                    };

                    if matches!(after, AfterPump::Stop) {
                        return;
                    }
                    if up_since.elapsed() >= BACKOFF_RESET_AFTER {
                        backoff = MIN_BACKOFF;
                    }
                }
                Err(error) => {
                    tracing::warn!(url = %self.url, %error, "upstream connect failed");
                }
            }
            // Sleep the backoff this attempt earned before growing it, so the
            // wait a caller actually experiences doubles from one second as
            // documented, rather than starting at its double.
            tokio::time::sleep(backoff).await;
            backoff = grow(backoff);
        }
    }

    /// Puts back on the new socket every frame `on_connect` asks for, and
    /// answers `true` only if all of them got there: a partial reissue leaves
    /// the connection subscribed to less than the caller believes, which is a
    /// bridge that reads as up and delivers nothing.
    async fn reissue_all(&self, ws: &mut WsStream, on_connect: &impl Fn() -> Vec<String>) -> bool {
        for frame in on_connect() {
            if ws.send(Message::text(frame)).await.is_err() {
                return false;
            }
        }
        true
    }

    fn drain_outbox(&mut self) {
        while self.outbox_rx.try_recv().is_ok() {}
    }
}

#[cfg(test)]
mod tests;
