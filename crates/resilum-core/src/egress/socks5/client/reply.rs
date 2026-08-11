//! Draining the backend's greeting and CONNECT reply.

use tokio::sync::mpsc::UnboundedReceiver;

use crate::egress::socks5::parse::read_at_least;
use crate::egress::socks5::{ATYP_DOMAIN, ATYP_V4, ATYP_V6, METHOD_NO_AUTH, SocksError, VER};
use crate::link::LinkMsg;

/// Returns whatever the peer streamed past the reply, so the caller can flush
/// it before pumping.
pub(super) async fn read_reply(
    from_link: &mut UnboundedReceiver<LinkMsg>,
) -> Result<Vec<u8>, SocksError> {
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
    use super::*;
    use crate::egress::socks5::{AUTH_NO_AUTH, REPLY_HOST_UNREACHABLE, REPLY_OK};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn reply_is_drained_and_trailing_bytes_are_returned() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(LinkMsg::Data(AUTH_NO_AUTH.to_vec())).unwrap();
        let mut reply = REPLY_OK.to_vec();
        reply.extend_from_slice(b"banner");
        tx.send(LinkMsg::Data(reply)).unwrap();
        drop(tx);
        assert_eq!(read_reply(&mut rx).await.unwrap(), b"banner");
    }

    #[tokio::test]
    async fn refused_reply_is_reported() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(LinkMsg::Data(AUTH_NO_AUTH.to_vec())).unwrap();
        tx.send(LinkMsg::Data(REPLY_HOST_UNREACHABLE.to_vec()))
            .unwrap();
        drop(tx);
        assert!(matches!(
            read_reply(&mut rx).await,
            Err(SocksError::Refused(0x04))
        ));
    }
}
