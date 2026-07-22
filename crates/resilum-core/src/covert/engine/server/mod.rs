//! Server engine: demux by session id, unseal on handshake, respond over the
//! same carrier. Bound to carriers whose reply address is an `IpAddr`.

mod respond;

use std::io;
use std::net::IpAddr;
use std::time::Duration;

use leviculum_std::api::Identity;

use super::datagram::{self, HEADER_LEN, Kind, overhead};
use super::session::{Session, SessionTable};
use crate::covert::carrier::CarrierServer;

const WINDOW: usize = 8;

pub type ServerOutput = Box<dyn FnMut(u32, &[u8]) + Send>;

pub struct ServerEngine<S>
where
    S: CarrierServer<ReplyTo = IpAddr>,
{
    carrier: S,
    identity: Identity,
    on_output: ServerOutput,
    tag: usize,
    pub table: SessionTable,
}

impl<S> ServerEngine<S>
where
    S: CarrierServer<ReplyTo = IpAddr>,
{
    pub fn new<F>(
        carrier: S,
        identity: Identity,
        on_output: F,
        session_ttl: Option<Duration>,
    ) -> Self
    where
        F: FnMut(u32, &[u8]) + Send + 'static,
    {
        let tag = carrier.tag_len();
        let size_for =
            Box::new(move |_reply: Option<IpAddr>| 1024usize.saturating_sub(overhead(tag)));
        let table = match session_ttl {
            Some(ttl) => SessionTable::with_ttl(size_for, WINDOW, ttl.as_secs_f64()),
            None => SessionTable::new(size_for, WINDOW),
        };
        Self {
            carrier,
            identity,
            on_output: Box::new(on_output),
            tag,
            table,
        }
    }

    pub fn broadcast(&mut self, data: &[u8]) {
        for s in self.table.established() {
            s.send.write(data);
        }
    }

    pub fn poll(&mut self, now: f64) {
        self.table.expire(now);
    }

    /// Ingest a raw wire payload already extracted from the carrier by
    /// [`CarrierServer::recv_request`].
    pub fn on_received(&mut self, reply_to: IpAddr, wire: &[u8], now: f64) -> io::Result<()> {
        let Some((session_id, _seq, _ack, kind_byte)) = datagram::peek_header(wire) else {
            return Ok(());
        };
        let Some(kind) = Kind::from_u8(kind_byte) else {
            return Ok(());
        };
        if kind == Kind::Handshake {
            return respond::open(self, session_id, &wire[HEADER_LEN..], reply_to, now);
        }
        let key = match self.table.get(session_id, now, Some(reply_to)).key.clone() {
            Some(k) => k,
            None => return Ok(()),
        };
        let Some(dg) = datagram::unpack(wire, &key, self.tag) else {
            return Ok(());
        };
        let out = {
            let s = self.table.get(session_id, now, Some(reply_to));
            s.reply_to = Some(reply_to);
            s.send.ack(dg.ack, now);
            (dg.kind == Kind::Data && !dg.payload.is_empty())
                .then(|| s.recv.feed(dg.seq, dg.payload))
        };
        if let Some(bytes) = out
            && !bytes.is_empty()
        {
            (self.on_output)(session_id, &bytes);
        }
        respond::respond(self, session_id, now)
    }

    pub fn session(&mut self, session_id: u32, now: f64, reply_to: IpAddr) -> &mut Session {
        self.table.get(session_id, now, Some(reply_to))
    }

    pub fn count(&self) -> usize {
        self.table.count()
    }
}
