//! Server-side per-client state, keyed by the datagram's session id.

use std::collections::HashMap;
use std::net::IpAddr;

use super::framing::{RecvBuffer, SendBuffer};

const DEFAULT_TTL_SECS: f64 = 300.0;
pub(in crate::covert::engine) const SESSIONS_HELD_AT_ONCE: usize = 256;

pub struct Session {
    pub send: SendBuffer,
    pub recv: RecvBuffer,
    pub reply_to: Option<IpAddr>,
    pub last_seen: f64,
    pub key: Option<Vec<u8>>,
}

/// Payload budget for the SendBuffer of a session addressing `reply_to`. The
/// carrier decides how many bytes fit; the SessionTable calls this on `open`.
pub type SizeFor = Box<dyn Fn(IpAddr) -> usize + Send + Sync>;

pub struct SessionTable {
    size_for: SizeFor,
    window: usize,
    ttl: f64,
    capacity: usize,
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
            capacity: SESSIONS_HELD_AT_ONCE,
            sessions: HashMap::new(),
        }
    }

    pub fn already_open(&mut self, session_id: u32, now: f64) -> Option<&mut Session> {
        let s = self.sessions.get_mut(&session_id)?;
        s.last_seen = now;
        Some(s)
    }

    pub fn open_unless_full(
        &mut self,
        session_id: u32,
        now: f64,
        reply_to: IpAddr,
    ) -> Option<&mut Session> {
        if !self.sessions.contains_key(&session_id) && self.sessions.len() >= self.capacity {
            return None;
        }
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
        Some(s)
    }

    pub fn key_of(&self, session_id: u32) -> Option<Vec<u8>> {
        self.sessions.get(&session_id)?.key.clone()
    }

    pub fn already_belongs_to_another_key(&self, session_id: u32, offered: &[u8]) -> bool {
        self.key_of(session_id)
            .is_some_and(|established| !super::datagram::ct_eq(&established, offered))
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
