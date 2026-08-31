use super::{payload_per_fragment, spec};

#[derive(Debug, PartialEq, Eq)]
pub enum Arrived {
    Packet(Vec<u8>),
    StillWaiting,
    Refused,
}

#[derive(Default)]
pub struct Reassembly {
    holding: Vec<u8>,
    expecting: u16,
    next: u16,
    started_ms: u64,
}

impl Reassembly {
    pub fn process(&mut self, fragment: &[u8], writable: usize, now_ms: u64) -> Arrived {
        let Some(head) = Head::read(fragment, writable) else {
            self.forget();
            return Arrived::Refused;
        };
        if self.stalled(now_ms) {
            self.forget();
        }
        match head.kind {
            spec::TYPE_LONE_ACCEPTED_NEVER_SENT => Arrived::Packet(head.payload.to_vec()),
            spec::TYPE_START => {
                self.begin(&head, now_ms);
                self.complete_if_whole(&head)
            }
            _ if head.seq != self.next || head.total != self.expecting => {
                self.forget();
                Arrived::Refused
            }
            _ => {
                self.holding.extend_from_slice(head.payload);
                self.next += 1;
                self.complete_if_whole(&head)
            }
        }
    }

    fn begin(&mut self, head: &Head<'_>, now_ms: u64) {
        self.holding.clear();
        self.holding.extend_from_slice(head.payload);
        self.expecting = head.total;
        self.next = 1;
        self.started_ms = now_ms;
    }

    fn complete_if_whole(&mut self, head: &Head<'_>) -> Arrived {
        if self.next < head.total {
            return Arrived::StillWaiting;
        }
        let packet = std::mem::take(&mut self.holding);
        self.forget();
        Arrived::Packet(packet)
    }

    fn stalled(&self, now_ms: u64) -> bool {
        self.expecting != 0
            && now_ms.saturating_sub(self.started_ms) > spec::ABANDON_REASSEMBLY_AFTER_MS
    }

    fn forget(&mut self) {
        self.holding.clear();
        self.expecting = 0;
        self.next = 0;
    }
}

struct Head<'a> {
    kind: u8,
    seq: u16,
    total: u16,
    payload: &'a [u8],
}

impl<'a> Head<'a> {
    fn read(fragment: &'a [u8], writable: usize) -> Option<Self> {
        let (head, payload) = fragment.split_at_checked(spec::FRAGMENT_HEADER_LEN)?;
        let kind = head[0];
        let seq = u16::from_be_bytes([head[1], head[2]]);
        let total = u16::from_be_bytes([head[3], head[4]]);
        let sane = kind <= spec::TYPE_END
            && total > 0
            && seq < total
            && usize::from(total) <= fragments_for_the_largest_packet(writable);
        sane.then_some(Self {
            kind,
            seq,
            total,
            payload,
        })
    }
}

fn fragments_for_the_largest_packet(writable: usize) -> usize {
    let room = payload_per_fragment(writable);
    if room == 0 {
        return 1;
    }
    spec::LARGEST_PACKET.div_ceil(room).max(1)
}
