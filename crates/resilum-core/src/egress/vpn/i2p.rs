//! I2P outbound over a SAM v3 STREAM session — one session per attachment,
//! built lazily on the first `.i2p` flow, then dialled per flow. Talks to a
//! local i2pd (SAM defaults to 127.0.0.1:7656).

use std::io;

use tokio::sync::Mutex;
use yosemite::{Session, SessionOptions, Stream, style};

#[derive(Default)]
pub struct I2pConduit {
    session: Mutex<Option<Session<style::Stream>>>,
}

impl I2pConduit {
    /// The first call builds the session (and its tunnels), which is slow; the
    /// lock serialises concurrent dials.
    pub(crate) async fn connect(&self, host: &str) -> io::Result<Stream> {
        let mut session = self.session.lock().await;
        if session.is_none() {
            *session = Some(
                Session::<style::Stream>::new(SessionOptions::default())
                    .await
                    .map_err(|e| io::Error::other(format!("i2p session: {e}")))?,
            );
        }
        session
            .as_mut()
            .expect("session initialised above")
            .connect(host)
            .await
            .map_err(|e| io::Error::other(format!("i2p connect: {e}")))
    }
}
