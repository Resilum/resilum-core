//! The receiving half: reorder into a contiguous stream.

use std::collections::BTreeMap;

pub struct RecvBuffer {
    expected: u32,
    buf: BTreeMap<u32, Vec<u8>>,
}

impl RecvBuffer {
    pub fn new() -> Self {
        Self {
            expected: 1,
            buf: BTreeMap::new(),
        }
    }

    pub fn ack(&self) -> u32 {
        self.expected - 1
    }

    /// Ingest one chunk, returning contiguous in-order bytes ready to deliver.
    pub fn feed(&mut self, seq: u32, payload: Vec<u8>) -> Vec<u8> {
        if seq < self.expected {
            return Vec::new();
        }
        self.buf.insert(seq, payload);
        let mut out = Vec::new();
        while let Some(chunk) = self.buf.remove(&self.expected) {
            out.extend(chunk);
            self.expected += 1;
        }
        out
    }
}

impl Default for RecvBuffer {
    fn default() -> Self {
        Self::new()
    }
}
