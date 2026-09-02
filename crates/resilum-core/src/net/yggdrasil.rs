use std::net::{IpAddr, Ipv6Addr};

/// Locally-assigned Yggdrasil IPv6, or `None` if no `ygg0`-style interface is
/// up. Yggdrasil owns `200::/7` so the interface name is irrelevant — matches
/// any platform (`ygg0` on Linux, `tun*`/`utun*` on Android/iOS/macOS).
pub fn yggdrasil_local_ipv6() -> Option<Ipv6Addr> {
    #[cfg(target_os = "linux")]
    if let Some(v6) = yggdrasil_ipv6_from_proc() {
        return Some(v6);
    }
    if_addrs::get_if_addrs()
        .ok()?
        .into_iter()
        .find_map(|i| match i.ip() {
            IpAddr::V6(v6) if is_yggdrasil_range(&v6) => Some(v6),
            _ => None,
        })
}

/// musl's `getifaddrs` skips the tun, so the table is read directly. Each line
/// starts with a 32-hex, colon-less address.
#[cfg(target_os = "linux")]
fn yggdrasil_ipv6_from_proc() -> Option<Ipv6Addr> {
    ygg_addr_in_if_inet6(&std::fs::read_to_string("/proc/net/if_inet6").ok()?)
}

#[cfg(target_os = "linux")]
fn ygg_addr_in_if_inet6(table: &str) -> Option<Ipv6Addr> {
    table.lines().find_map(|line| {
        let v6 = Ipv6Addr::from(parse_hex16(line.split_whitespace().next()?)?);
        is_yggdrasil_range(&v6).then_some(v6)
    })
}

#[cfg(target_os = "linux")]
fn parse_hex16(hex: &str) -> Option<[u8; 16]> {
    if hex.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (byte, pair) in out.iter_mut().zip(hex.as_bytes().as_chunks::<2>().0) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        *byte = (hi * 16 + lo) as u8;
    }
    Some(out)
}

fn is_yggdrasil_range(ip: &Ipv6Addr) -> bool {
    // 200::/7 → first 7 bits are 0000_001, i.e. first octet is 0x02 or 0x03.
    (ip.octets()[0] & 0xfe) == 0x02
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn parses_ygg_address_from_proc_hex() {
        // An ygg 200::/7 global followed by ygg0's link-local; the global must
        // be the one picked.
        let table = "02010000000000000000000000000001 01 07 00 80     ygg0\n\
                     fe800000000000000000000000000002 01 40 20 80     ygg0\n";
        assert_eq!(
            ygg_addr_in_if_inet6(table),
            Some("201::1".parse::<Ipv6Addr>().unwrap())
        );
        assert!(is_yggdrasil_range(&"201::1".parse().unwrap()));
        assert!(!is_yggdrasil_range(&"fe80::2".parse().unwrap()));
        assert!(parse_hex16(&"z".repeat(32)).is_none(), "non-hex rejected");
        assert!(parse_hex16("0201").is_none(), "wrong length rejected");
    }
}
