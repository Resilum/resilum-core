use crate::link::LinkMsg;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{ATYP_DOMAIN, ATYP_V4, ATYP_V6, CMD_CONNECT, METHOD_NO_AUTH, SocksError, VER};

#[derive(Debug, PartialEq, Eq)]
pub struct ConnectRequest {
    pub host: String,
    pub port: u16,
    /// SOCKS5 clients may pipeline application bytes right after CONNECT;
    /// forward these upstream before pumping.
    pub leftover: Vec<u8>,
}

pub async fn handshake(
    from_link: &mut UnboundedReceiver<LinkMsg>,
) -> Result<ConnectRequest, SocksError> {
    let mut buf = Vec::new();
    read_at_least(from_link, &mut buf, 2).await?;
    if buf[0] != VER {
        return Err(SocksError::BadVersion(buf[0]));
    }
    let n = buf[1] as usize;
    read_at_least(from_link, &mut buf, 2 + n).await?;
    if !buf[2..2 + n].contains(&METHOD_NO_AUTH) {
        return Err(SocksError::NoAcceptableAuth);
    }
    buf.drain(..2 + n);

    read_at_least(from_link, &mut buf, 4).await?;
    if buf[0] != VER {
        return Err(SocksError::BadVersion(buf[0]));
    }
    if buf[1] != CMD_CONNECT {
        return Err(SocksError::UnsupportedCommand(buf[1]));
    }
    let atyp = buf[3];
    let addr_len = match atyp {
        ATYP_V4 => 4,
        ATYP_V6 => 16,
        ATYP_DOMAIN => {
            read_at_least(from_link, &mut buf, 5).await?;
            buf[4] as usize + 1
        }
        _ => return Err(SocksError::UnsupportedAtyp(atyp)),
    };
    let end = 4 + addr_len + 2;
    read_at_least(from_link, &mut buf, end).await?;

    let host = decode_host(atyp, &buf[4..end - 2])?;
    let port = u16::from_be_bytes([buf[end - 2], buf[end - 1]]);
    let leftover = buf.split_off(end);
    Ok(ConnectRequest {
        host,
        port,
        leftover,
    })
}

fn decode_host(atyp: u8, addr: &[u8]) -> Result<String, SocksError> {
    match atyp {
        ATYP_V4 => Ok(format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])),
        ATYP_V6 => {
            let mut segs = [0u16; 8];
            for (i, seg) in segs.iter_mut().enumerate() {
                *seg = u16::from_be_bytes([addr[i * 2], addr[i * 2 + 1]]);
            }
            Ok(std::net::Ipv6Addr::from(segs).to_string())
        }
        ATYP_DOMAIN => std::str::from_utf8(&addr[1..])
            .map(str::to_owned)
            .map_err(|_| SocksError::BadDomain),
        _ => Err(SocksError::UnsupportedAtyp(atyp)),
    }
}

pub(super) async fn read_at_least(
    from_link: &mut UnboundedReceiver<LinkMsg>,
    buf: &mut Vec<u8>,
    need: usize,
) -> Result<(), SocksError> {
    while buf.len() < need {
        match from_link.recv().await {
            Some(LinkMsg::Data(bytes)) => buf.extend_from_slice(&bytes),
            Some(LinkMsg::Established) => {}
            Some(LinkMsg::Closed) | None => return Err(SocksError::LinkClosed),
        }
    }
    Ok(())
}
