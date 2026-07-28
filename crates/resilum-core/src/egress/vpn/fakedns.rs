//! FakeDNS: answer A queries locally with synthetic addresses from a reserved
//! pool, keeping a fakeip↔hostname map so the router can recover the name when
//! the app later connects to that address. DNS never leaves the device.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Mutex;

use simple_dns::rdata::RData;
use simple_dns::{CLASS, Packet, QTYPE, ResourceRecord, TYPE};

/// 198.18.0.0/15 (RFC 2544 benchmarking range) never appears on the real
/// network, so its addresses are safe to hand out as synthetic.
const POOL_BASE: u32 = u32::from_be_bytes([198, 18, 0, 0]);
const POOL_SIZE: u32 = 1 << 17;
const TTL: u32 = 30;

#[derive(Default)]
pub(crate) struct FakeDns {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    next: u32,
    by_name: HashMap<String, Ipv4Addr>,
    by_ip: HashMap<Ipv4Addr, String>,
}

impl FakeDns {
    pub(crate) fn is_fake(ip: IpAddr) -> bool {
        matches!(ip, IpAddr::V4(v4) if (POOL_BASE..POOL_BASE + POOL_SIZE).contains(&u32::from(v4)))
    }

    pub(crate) fn resolve(&self, ip: Ipv4Addr) -> Option<String> {
        self.inner.lock().expect("fakedns").by_ip.get(&ip).cloned()
    }

    /// Answer a query: an A question gets a synthetic address, anything else
    /// NODATA. `None` if the query cannot be read.
    pub(crate) fn answer(&self, query: &[u8]) -> Option<Vec<u8>> {
        let packet = Packet::parse(query).ok()?;
        let question = packet.questions.first()?;
        let is_a = matches!(question.qtype, QTYPE::TYPE(TYPE::A));
        let qname = question.qname.clone();
        let mut reply = packet.into_reply();
        if is_a && let Some(ip) = self.allocate(qname.to_string()) {
            reply.answers.push(ResourceRecord::new(
                qname,
                CLASS::IN,
                TTL,
                RData::A(ip.into()),
            ));
        }
        reply.build_bytes_vec().ok()
    }

    fn allocate(&self, host: String) -> Option<Ipv4Addr> {
        let mut inner = self.inner.lock().expect("fakedns");
        if let Some(&ip) = inner.by_name.get(&host) {
            return Some(ip);
        }
        if inner.next >= POOL_SIZE {
            return None;
        }
        let ip = Ipv4Addr::from(POOL_BASE + inner.next);
        inner.next += 1;
        inner.by_name.insert(host.clone(), ip);
        inner.by_ip.insert(ip, host);
        Some(ip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_dns::{Name, QCLASS, Question};

    fn query(host: &str, qtype: TYPE) -> Vec<u8> {
        let mut packet = Packet::new_query(0x1234);
        packet.questions.push(Question::new(
            Name::new(host).unwrap(),
            QTYPE::TYPE(qtype),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.build_bytes_vec().unwrap()
    }

    fn answered_a(dns: &FakeDns, host: &str) -> Ipv4Addr {
        let reply = dns.answer(&query(host, TYPE::A)).unwrap();
        let parsed = Packet::parse(&reply).unwrap();
        let RData::A(a) = &parsed.answers[0].rdata else {
            panic!("expected an A record");
        };
        Ipv4Addr::from(a.address)
    }

    #[test]
    fn a_query_yields_a_pool_address_that_resolves_back() {
        let dns = FakeDns::default();
        let ip = answered_a(&dns, "example.com");
        assert!(FakeDns::is_fake(ip.into()));
        assert_eq!(dns.resolve(ip).as_deref(), Some("example.com"));
    }

    #[test]
    fn a_host_keeps_the_same_address() {
        let dns = FakeDns::default();
        assert_eq!(answered_a(&dns, "a.example"), answered_a(&dns, "a.example"));
        assert_ne!(answered_a(&dns, "a.example"), answered_a(&dns, "b.example"));
    }

    #[test]
    fn aaaa_query_is_nodata() {
        let dns = FakeDns::default();
        let reply = dns.answer(&query("example.com", TYPE::AAAA)).unwrap();
        assert!(Packet::parse(&reply).unwrap().answers.is_empty());
    }

    #[test]
    fn a_malformed_query_is_rejected() {
        assert!(
            FakeDns::default()
                .answer(b"\xff not a dns packet")
                .is_none()
        );
    }
}
