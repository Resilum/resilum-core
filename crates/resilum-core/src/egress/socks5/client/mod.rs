//! Client-side SOCKS5 CONNECT toward the embedded egress backend (no-auth).
//! The backend reads the greeting and the CONNECT request before it replies, so
//! both are pipelined.

mod reply;

use std::net::SocketAddr;

use leviculum_std::api::LinkHandle;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::egress::socks5::{
    ATYP_DOMAIN, ATYP_V4, ATYP_V6, CMD_CONNECT, METHOD_NO_AUTH, SocksError, VER,
};
use crate::link::LinkMsg;

/// What to ask the egress to reach: a literal address, or a hostname the egress
/// resolves itself (used when the app dialled a FakeDNS synthetic address).
#[non_exhaustive]
pub enum Target {
    Addr(SocketAddr),
    Domain(String, u16),
}

pub async fn connect(
    handle: &LinkHandle,
    from_link: &mut UnboundedReceiver<LinkMsg>,
    target: &Target,
) -> Result<Vec<u8>, SocksError> {
    handle
        .send(&encode_request(target)?)
        .await
        .map_err(|_| SocksError::LinkClosed)?;
    reply::read_reply(from_link).await
}

fn encode_request(target: &Target) -> Result<Vec<u8>, SocksError> {
    let mut out = vec![VER, 0x01, METHOD_NO_AUTH, VER, CMD_CONNECT, 0x00];
    let port = match target {
        Target::Addr(SocketAddr::V4(v4)) => {
            out.push(ATYP_V4);
            out.extend_from_slice(&v4.ip().octets());
            v4.port()
        }
        Target::Addr(SocketAddr::V6(v6)) => {
            out.push(ATYP_V6);
            out.extend_from_slice(&v6.ip().octets());
            v6.port()
        }
        Target::Domain(host, port) => {
            let len = u8::try_from(host.len()).map_err(|_| SocksError::BadDomain)?;
            out.push(ATYP_DOMAIN);
            out.push(len);
            out.extend_from_slice(host.as_bytes());
            *port
        }
    };
    out.extend_from_slice(&port.to_be_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::egress::socks5::parse::handshake;
    use tokio::sync::mpsc;

    async fn parse_request(target: Target) -> (String, u16) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(LinkMsg::Data(encode_request(&target).unwrap()))
            .unwrap();
        drop(tx);
        let req = handshake(&mut rx).await.unwrap();
        (req.host, req.port)
    }

    #[tokio::test]
    async fn addr_request_round_trips_through_the_server_parser_v4() {
        let (host, port) = parse_request(Target::Addr("1.1.1.1:443".parse().unwrap())).await;
        assert_eq!(host, "1.1.1.1");
        assert_eq!(port, 443);
    }

    #[tokio::test]
    async fn addr_request_round_trips_through_the_server_parser_v6() {
        let (host, port) =
            parse_request(Target::Addr("[2606:4700:4700::1111]:53".parse().unwrap())).await;
        assert_eq!(host, "2606:4700:4700::1111");
        assert_eq!(port, 53);
    }

    #[tokio::test]
    async fn domain_request_round_trips_through_the_server_parser() {
        let (host, port) = parse_request(Target::Domain("example.com".into(), 80)).await;
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
    }
}
