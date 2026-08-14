//! The address a peer advertises, on the wire and back. Binary rather than
//! text: base32 costs 60% more than the bytes it spells out.

use std::net::Ipv6Addr;

use crate::config::EndpointFormat;

pub(super) fn encode_endpoint(host: &str, port: u16, format: &EndpointFormat) -> Option<Vec<u8>> {
    let mut out = match format {
        EndpointFormat::BracketedIpv6 => host.parse::<Ipv6Addr>().ok()?.octets().to_vec(),
        EndpointFormat::Base32 { suffix, label_len } => {
            let label = host.strip_suffix(suffix.as_str())?;
            let decoded = data_encoding::BASE32_NOPAD
                .decode(label.to_ascii_uppercase().as_bytes())
                .ok()?;
            (decoded.len() == *label_len).then_some(decoded)?
        }
    };
    out.extend_from_slice(&port.to_be_bytes());
    Some(out)
}

pub(super) fn parse_endpoint(payload: &[u8], format: &EndpointFormat) -> Option<(String, u16)> {
    let (address, port) = payload.split_at_checked(payload.len().checked_sub(2)?)?;
    let port = u16::from_be_bytes([port[0], port[1]]);
    if port == 0 {
        return None;
    }
    let host = match format {
        EndpointFormat::BracketedIpv6 => {
            let octets: [u8; 16] = address.try_into().ok()?;
            Ipv6Addr::from(octets).to_string()
        }
        EndpointFormat::Base32 { suffix, label_len } => {
            if address.len() != *label_len {
                return None;
            }
            format!(
                "{}{suffix}",
                data_encoding::BASE32_NOPAD
                    .encode(address)
                    .to_ascii_lowercase()
            )
        }
    };
    Some((host, port))
}

#[cfg(test)]
mod tests;
