//! Client engine: handshake a per-link key, then pump ARQ over the carrier.
//! One instance per (carrier, server, session) tuple.

use std::io;
use std::time::Duration;

use leviculum_std::api::Identity;

use super::datagram::{self, Datagram, Kind, overhead};
use super::framing::{RecvBuffer, SendBuffer};
use super::keyx;
use super::poll::AdaptivePoll;
use crate::covert::carrier::CarrierClient;

const HANDSHAKE_INTERVAL_SECS: f64 = 5.0;
const WINDOW: usize = 8;

pub type ClientOutput = Box<dyn FnMut(&[u8]) + Send>;

pub struct ClientEngine<C: CarrierClient> {
    carrier: C,
    server: Identity,
    sid: u32,
    on_output: ClientOutput,
    tag: usize,
    key: [u8; keyx::SESSION_KEY_LEN],
    send: SendBuffer,
    recv: RecvBuffer,
    poll: AdaptivePoll,
    last_poll: f64,
    last_hs: f64,
    established: bool,
    had_traffic: bool,
}

impl<C: CarrierClient> ClientEngine<C> {
    pub fn new<F>(
        carrier: C,
        server: Identity,
        sid: u32,
        on_output: F,
        min_poll: Duration,
        max_poll: Duration,
    ) -> Self
    where
        F: FnMut(&[u8]) + Send + 'static,
    {
        let tag = carrier.tag_len();
        let payload = carrier.capacity().saturating_sub(overhead(tag));
        Self {
            carrier,
            server,
            sid,
            on_output: Box::new(on_output),
            tag,
            key: keyx::new_session_key(),
            send: SendBuffer::new(payload, WINDOW),
            recv: RecvBuffer::new(),
            poll: AdaptivePoll::new(min_poll, max_poll),
            last_poll: f64::MIN,
            last_hs: f64::MIN,
            established: false,
            had_traffic: false,
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        self.send.write(data);
        self.had_traffic = true;
    }

    /// Feed an inbound raw payload back into the engine.
    pub fn on_received(&mut self, raw: &[u8], now: f64) -> io::Result<()> {
        let Some(dg) = datagram::unpack(raw, &self.key, self.tag) else {
            return Ok(());
        };
        self.established = true;
        self.send.ack(dg.ack, now);
        if dg.kind == Kind::Data && !dg.payload.is_empty() {
            let out = self.recv.feed(dg.seq, dg.payload);
            if !out.is_empty() {
                self.had_traffic = true;
                (self.on_output)(&out);
            }
        }
        Ok(())
    }

    /// Drive one tick: emit handshake retries, ready chunks, and keepalive polls.
    pub fn poll(&mut self, now: f64) -> io::Result<()> {
        if !self.established && now - self.last_hs >= HANDSHAKE_INTERVAL_SECS {
            let token = keyx::seal(&self.server, &self.key)
                .ok_or_else(|| io::Error::other("seal session key"))?;
            let hs = Datagram {
                session: self.sid,
                seq: 0,
                ack: 0,
                kind: Kind::Handshake,
                payload: token,
            };
            self.carrier.send_request(&datagram::pack(&hs, b"", 0))?;
            self.last_hs = now;
        }
        let due = now - self.last_poll >= self.poll.interval().as_secs_f64();
        let pkts = self.send.ready(now);
        if !pkts.is_empty() {
            for (seq, payload) in pkts {
                self.emit(Datagram {
                    session: self.sid,
                    seq,
                    ack: self.recv.ack(),
                    kind: Kind::Data,
                    payload,
                })?;
            }
            self.last_poll = now;
        } else if due {
            self.emit(Datagram {
                session: self.sid,
                seq: 0,
                ack: self.recv.ack(),
                kind: Kind::Poll,
                payload: Vec::new(),
            })?;
            self.last_poll = now;
        }
        self.poll.observe(self.had_traffic);
        self.had_traffic = false;
        Ok(())
    }

    fn emit(&self, dg: Datagram) -> io::Result<()> {
        let wire = datagram::pack(&dg, &self.key, self.tag);
        self.carrier.send_request(&wire)
    }
}
