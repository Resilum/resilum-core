//! Reads and writes one live connection until it ends.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until, timeout};
use tokio_tungstenite::tungstenite::protocol::frame::Utf8Bytes;
use tokio_tungstenite::tungstenite::{Bytes, Message};

use super::WsStream;
use crate::upstream::proto;

/// How long a relay may say nothing before we ask whether it is there, and
/// how long the answer may take. `answer` bounds a write too, this being a
/// single task.
///
/// A NAT or firewall that drops an idle mapping leaves a socket that reads as
/// open and never delivers again. Without this the pump waits in `select!`
/// for ever with `up` still true: publishes wait out their deadline against a
/// relay that cannot hear them, inbound stops, and nothing ever re-dials.
pub(crate) struct Deadlines {
    pub(crate) idle: Duration,
    pub(crate) answer: Duration,
}

impl Default for Deadlines {
    fn default() -> Self {
        Self {
            idle: Duration::from_secs(60),
            answer: Duration::from_secs(20),
        }
    }
}

/// What the reconnect loop does once this connection is over.
pub(super) enum AfterPump {
    Redial,
    Stop,
}

/// Every arm below is cancel-safe, which is what makes discarding two of the
/// three futures on each iteration harmless: `StreamExt::next` leaves a
/// partly-read frame in the stream's own buffer rather than in the dropped
/// future, `recv` only removes a value it has already handed back, and
/// `sleep_until` is a deadline, not an elapsed count.
pub(super) async fn pump(
    url: &str,
    ws: &mut WsStream,
    incoming: &mpsc::UnboundedSender<proto::Incoming>,
    outbox_rx: &mut mpsc::UnboundedReceiver<Utf8Bytes>,
    deadlines: &Deadlines,
) -> AfterPump {
    let mut heard = Instant::now();
    let mut asked = false;
    loop {
        // Anchored to the last frame heard rather than to this iteration, so
        // a busy outbox cannot keep a dead peer looking alive.
        let grace = if asked {
            deadlines.answer
        } else {
            Duration::ZERO
        };
        let due = heard + deadlines.idle + grace;
        tokio::select! {
            frame = ws.next() => {
                match frame {
                    Some(Ok(message)) => {
                        heard = Instant::now();
                        asked = false;
                        // Ping, pong, binary and close each count as the peer
                        // being there, the only question the idle deadline
                        // asks; only text carries protocol, so the rest are
                        // deliberately dropped once counted.
                        if let Message::Text(text) = message {
                            match proto::parse_incoming(&text) {
                                Some(parsed) => if incoming.send(parsed).is_err() {
                                    return AfterPump::Stop;
                                },
                                None => tracing::debug!(frame = %preview(&text), "unparsed relay frame"),
                            }
                        }
                    }
                    Some(Err(error)) => {
                        tracing::warn!(url, %error, "upstream connection lost");
                        return AfterPump::Redial;
                    }
                    None => {
                        tracing::warn!(url, "upstream connection closed");
                        return AfterPump::Redial;
                    }
                }
            }
            frame = outbox_rx.recv() => {
                let Some(frame) = frame else {
                    // Every `Upstream` handle is gone, so nothing can be queued
                    // again and `recv` answers `None` at once for ever:
                    // redialling would spin on connections this arm always
                    // wins the `select!` on, reading nothing.
                    tracing::info!(url, "no upstream handles left; leaving the relay");
                    return AfterPump::Stop;
                };
                if !sent(ws, Message::Text(frame), deadlines).await {
                    return AfterPump::Redial;
                }
            }
            () = sleep_until(due) => {
                if asked {
                    tracing::warn!(url, "upstream stopped answering; dropping the connection");
                    return AfterPump::Redial;
                }
                if !sent(ws, Message::Ping(Bytes::new()), deadlines).await {
                    return AfterPump::Redial;
                }
                asked = true;
            }
        }
    }
}

/// A write that outlasts `answer` means the socket is not draining, and
/// waiting it out would stop inbound reading and the deadline that exists to
/// catch exactly that. The abandoned write leaves the sink part-way through a
/// frame, so `false` must end the connection rather than move on to the next
/// frame.
async fn sent(ws: &mut WsStream, message: Message, deadlines: &Deadlines) -> bool {
    matches!(
        timeout(deadlines.answer, ws.send(message)).await,
        Ok(Ok(()))
    )
}

fn preview(text: &str) -> &str {
    let end = text.char_indices().nth(64).map_or(text.len(), |(i, _)| i);
    &text[..end]
}
