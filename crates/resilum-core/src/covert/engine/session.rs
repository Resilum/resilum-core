//! Server-side per-client state, keyed by the datagram's session id.

use std::collections::HashMap;
use std::net::IpAddr;

use super::framing::{RecvBuffer, SendBuffer};

const DEFAULT_TTL_SECS: f64 = 300.0;

pub struct Session {
    pub send: SendBuffer,
    pub recv: RecvBuffer,
    pub reply_to: Option<IpAddr>,
    pub last_seen: f64,
    pub key: Option<Vec<u8>>,
}

/// Payload budget for the SendBuffer of a session addressing `reply_to`. The
/// carrier decides how many bytes fit; the SessionTable calls this on `get`.
pub type SizeFor = Box<dyn Fn(Option<IpAddr>) -> usize + Send + Sync>;

pub struct SessionTable {
    size_for: SizeFor,
    window: usize,
    ttl: f64,
    sessions: HashMap<u32, Session>,
}

impl SessionTable {
    pub fn new(size_for: SizeFor, window: usize) -> Self {
        Self::with_ttl(size_for, window, DEFAULT_TTL_SECS)
    }

    pub fn with_ttl(size_for: SizeFor, window: usize, ttl: f64) -> Self {
        Self {
            size_for,
            window,
            ttl,
            sessions: HashMap::new(),
        }
    }

    pub fn get(&mut self, session_id: u32, now: f64, reply_to: Option<IpAddr>) -> &mut Session {
        let window = self.window;
        let payload = (self.size_for)(reply_to);
        let s = self.sessions.entry(session_id).or_insert_with(|| Session {
            send: SendBuffer::new(payload, window),
            recv: RecvBuffer::new(),
            reply_to: None,
            last_seen: now,
            key: None,
        });
        s.last_seen = now;
        s
    }

    pub fn expire(&mut self, now: f64) {
        let ttl = self.ttl;
        self.sessions.retain(|_, s| now - s.last_seen <= ttl);
    }

    pub fn established(&mut self) -> impl Iterator<Item = &mut Session> {
        self.sessions.values_mut().filter(|s| s.key.is_some())
    }

    pub fn count(&self) -> usize {
        self.sessions.len()
    }
}
