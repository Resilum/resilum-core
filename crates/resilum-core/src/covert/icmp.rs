//! ICMP-echo carrier skeleton.
//!
//! Two send/recv modes:
//! - **`SOCK_DGRAM + IPPROTO_ICMP`** (Linux 2.6.39+, incl. Android with a plain
//!   `INTERNET` permission — no root, no raw): kernel-managed echo id, useful
//!   on mobile clients.
//! - **`SOCK_RAW + AF_PACKET + BPF`** (root / `CAP_NET_RAW`): bypasses
//!   netfilter so the kernel's own echo-reply does not race with ours; used
//!   on the server side that answers multiple clients.
//!
//! Full implementation is deferred; this holds the shape so the engine can
//! spawn it and the `Carrier` trait is honoured.

use super::carrier::{Carrier, Packet, Raw, ReplyTo, Semantics};

/// Payload budget per echo: default IPv4 MTU (1500) minus IPv4 (20) and ICMP (8)
/// headers.
const CAPACITY: usize = 1472;

pub struct IcmpCarrier {
    // Actual sockets, id, checksum state land here.
}

impl Default for IcmpCarrier {
    fn default() -> Self {
        Self::new()
    }
}

impl IcmpCarrier {
    pub fn new() -> Self {
        Self {}
    }
}

impl Carrier for IcmpCarrier {
    fn name(&self) -> &str {
        "icmp"
    }
    fn semantics(&self) -> Semantics {
        Semantics::Poll
    }
    fn capacity(&self) -> usize {
        CAPACITY
    }

    fn build_request(&self, _reply_to: Option<&ReplyTo>, _wire: &[u8]) -> Packet {
        unimplemented!("ICMP echo-request framing is TODO")
    }

    fn parse_request(&self, _raw: &Raw) -> Option<(ReplyTo, Vec<u8>)> {
        unimplemented!("ICMP echo-request deframing is TODO")
    }

    fn build_response(&self, _reply_to: &ReplyTo, _wire: &[u8]) -> Packet {
        unimplemented!("ICMP echo-reply framing is TODO")
    }

    fn parse_response(&self, _raw: &Raw) -> Option<Vec<u8>> {
        unimplemented!("ICMP echo-reply deframing is TODO")
    }

    fn send(&self, _packet: Packet) {
        unimplemented!("raw ICMP send is TODO")
    }
}
