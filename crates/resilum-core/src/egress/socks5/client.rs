//! Client-side SOCKS5 CONNECT toward the embedded egress backend (no-auth).
//! The backend reads the greeting and the CONNECT request before it replies, so
//! both are pipelined; the reply is fully drained and any bytes the peer already
//! streamed past it are returned so the caller can flush them before pumping.

use std::net::SocketAddr;

use leviculum_std::api::LinkHandle;
use tokio::sync::mpsc::UnboundedReceiver;

use super::parse::read_at_least;
use super::{ATYP_DOMAIN, ATYP_V4, ATYP_V6, CMD_CONNECT, METHOD_NO_AUTH, SocksError, VER};
use crate::link::LinkMsg;

pub async fn connect(
    handle: &LinkHandle,
    from_link: &mut UnboundedReceiver<LinkMsg>,
    dest: SocketAddr,
) -> Result<Vec<u8>, SocksError> {
    handle
        .send(&encode_request(dest))
        .await
        .map_err(|_| SocksError::LinkClosed)?;
    read_reply(from_link).await
}

fn encode_request(dest: SocketAddr) -> Vec<u8> {
    let mut out = vec![VER, 0x01, METHOD_NO_AUTH, VER, CMD_CONNECT, 0x00];
    match dest {
        SocketAddr::V4(v4) => {
            out.push(ATYP_V4);
            out.extend_from_slice(&v4.ip().octets());
        }
        SocketAddr::V6(v6) => {
            out.push(ATYP_V6);
            out.extend_from_slice(&v6.ip().octets());
        }
    }
    out.extend_from_slice(&dest.port().to_be_bytes());
    out
}

async fn read_reply(from_link: &mut UnboundedReceiver<LinkMsg>) -> Result<Vec<u8>, SocksError> {
    let mut buf = Vec::new();
    read_at_least(from_link, &mut buf, 2).await?;
    if buf[0] != VER {
        return Err(SocksError::BadVersion(buf[0]));
    }
    if buf[1] != METHOD_NO_AUTH {
        return Err(SocksError::NoAcceptableAuth);
    }
    buf.drain(..2);

    read_at_least(from_link, &mut buf, 4).await?;
    if buf[0] != VER {
        return Err(SocksError::BadVersion(buf[0]));
    }
    if buf[1] != 0x00 {
        return Err(SocksError::Refused(buf[1]));
    }
    let end = 4 + addr_len(from_link, &mut buf).await? + 2;
    read_at_least(from_link, &mut buf, end).await?;
    Ok(buf.split_off(end))
}

async fn addr_len(
    from_link: &mut UnboundedReceiver<LinkMsg>,
    buf: &mut Vec<u8>,
) -> Result<usize, SocksError> {
    match buf[3] {
        ATYP_V4 => Ok(4),
        ATYP_V6 => Ok(16),
        ATYP_DOMAIN => {
            read_at_least(from_link, buf, 5).await?;
            Ok(buf[4] as usize + 1)
        }
        other => Err(SocksError::UnsupportedAtyp(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::handshake;
    use super::*;
    use tokio::sync::mpsc;

    async fn parse_our_request(dest: SocketAddr) -> (String, u16) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(LinkMsg::Data(encode_request(dest))).unwrap();
        drop(tx);
        let req = handshake(&mut rx).await.unwrap();
        (req.host, req.port)
    }

    #[tokio::test]
    async fn request_round_trips_through_the_server_parser_v4() {
        let (host, port) = parse_our_request("1.1.1.1:443".parse().unwrap()).await;
        assert_eq!(host, "1.1.1.1");
        assert_eq!(port, 443);
    }

    #[tokio::test]
    async fn request_round_trips_through_the_server_parser_v6() {
        let (host, port) = parse_our_request("[2606:4700:4700::1111]:53".parse().unwrap()).await;
        assert_eq!(host, "2606:4700:4700::1111");
        assert_eq!(port, 53);
    }

    #[tokio::test]
    async fn reply_is_drained_and_trailing_bytes_are_returned() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(LinkMsg::Data(super::super::AUTH_NO_AUTH.to_vec()))
            .unwrap();
        let mut reply = super::super::REPLY_OK.to_vec();
        reply.extend_from_slice(b"banner");
        tx.send(LinkMsg::Data(reply)).unwrap();
        drop(tx);
        assert_eq!(read_reply(&mut rx).await.unwrap(), b"banner");
    }

    #[tokio::test]
    async fn refused_reply_is_reported() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(LinkMsg::Data(super::super::AUTH_NO_AUTH.to_vec()))
            .unwrap();
        tx.send(LinkMsg::Data(super::super::REPLY_HOST_UNREACHABLE.to_vec()))
            .unwrap();
        drop(tx);
        assert!(matches!(
            read_reply(&mut rx).await,
            Err(SocksError::Refused(0x04))
        ));
    }
}
