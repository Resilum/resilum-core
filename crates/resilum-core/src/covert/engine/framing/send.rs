//! The sending half: a send window, retransmits, and the RTT estimator.

use std::collections::BTreeMap;
use std::time::Duration;

use super::{RTO_INITIAL, clamp_rto};

const ALPHA: f64 = 0.125;
const BETA: f64 = 0.25;
const K: f64 = 4.0;

struct Unacked {
    payload: Vec<u8>,
    sent_at: f64,
    retransmitted: bool,
}

pub struct SendBuffer {
    payload_size: usize,
    window: usize,
    pending: Vec<u8>,
    next_seq: u32,
    acked: u32,
    unacked: BTreeMap<u32, Unacked>,
    srtt: Option<f64>,
    rttvar: f64,
    rto: Duration,
}

impl SendBuffer {
    pub fn new(payload_size: usize, window: usize) -> Self {
        Self {
            payload_size,
            window,
            pending: Vec::new(),
            next_seq: 1,
            acked: 0,
            unacked: BTreeMap::new(),
            srtt: None,
            rttvar: 0.0,
            rto: RTO_INITIAL,
        }
    }

    pub fn payload_size(&self) -> usize {
        self.payload_size
    }

    pub fn rto(&self) -> Duration {
        self.rto
    }

    pub fn write(&mut self, data: &[u8]) {
        self.pending.extend_from_slice(data);
    }

    pub fn has_unsent(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn ack(&mut self, ack_seq: u32, now: f64) {
        if ack_seq <= self.acked {
            return;
        }
        // Only fresh (non-retransmitted) samples update RTT.
        let mut sample: Option<f64> = None;
        for seq in (self.acked + 1)..=ack_seq {
            if let Some(chunk) = self.unacked.get(&seq)
                && !chunk.retransmitted
            {
                sample = Some(now - chunk.sent_at);
            }
        }
        self.acked = ack_seq;
        self.unacked.retain(|&seq, _| seq > ack_seq);
        if let Some(r) = sample {
            self.observe_rtt(r);
        }
    }

    fn observe_rtt(&mut self, r: f64) {
        match self.srtt {
            None => {
                self.srtt = Some(r);
                self.rttvar = r / 2.0;
            }
            Some(prev) => {
                self.rttvar = (1.0 - BETA) * self.rttvar + BETA * (prev - r).abs();
                self.srtt = Some((1.0 - ALPHA) * prev + ALPHA * r);
            }
        }
        let base = self.srtt.unwrap() + K * self.rttvar;
        self.rto = clamp_rto(Duration::from_secs_f64(base.max(0.0)));
    }

    /// Chunks to send now: retransmits with expired RTO, followed by fresh
    /// payloads until the send window fills.
    pub fn ready(&mut self, now: f64) -> Vec<(u32, Vec<u8>)> {
        let mut out = Vec::new();
        let mut resent = false;
        let rto_secs = self.rto.as_secs_f64();
        for (&seq, chunk) in self.unacked.iter_mut() {
            if now - chunk.sent_at >= rto_secs {
                chunk.sent_at = now;
                chunk.retransmitted = true;
                out.push((seq, chunk.payload.clone()));
                resent = true;
            }
        }
        if resent {
            self.rto = clamp_rto(self.rto * 2);
        }
        while !self.pending.is_empty()
            && (self.next_seq.saturating_sub(self.acked).saturating_sub(1)) < self.window as u32
        {
            let take = self.payload_size.min(self.pending.len());
            let payload: Vec<u8> = self.pending.drain(..take).collect();
            self.unacked.insert(
                self.next_seq,
                Unacked {
                    payload: payload.clone(),
                    sent_at: now,
                    retransmitted: false,
                },
            );
            out.push((self.next_seq, payload));
            self.next_seq += 1;
        }
        out
    }
}
