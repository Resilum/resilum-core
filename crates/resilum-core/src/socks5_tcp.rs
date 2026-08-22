//! Minimal SOCKS5 CONNECT server over a `TcpStream`: no-auth greeting, read the
//! CONNECT target, reply. Shared by the overlay SOCKS proxies (tor, yggdrasil);
//! each supplies its own backend to open the connection.

use std::io;
use std::net::IpAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const VER: u8 = 5;
const CMD_CONNECT: u8 = 1;
const ATYP_V4: u8 = 1;
const ATYP_DOMAIN: u8 = 3;
const ATYP_V6: u8 = 4;
const REP_CMD_NOT_SUPPORTED: u8 = 7;

#[cfg(any(feature = "arti", feature = "ygg"))]
pub(crate) const REP_OK: u8 = 0;
pub(crate) const REP_NO_EGRESS_TO_REACH_THE_INTERNET_THROUGH: u8 = 3;
pub(crate) const REP_EGRESS_DID_NOT_ANSWER: u8 = 4;
#[cfg(any(feature = "arti", feature = "ygg"))]
pub(crate) const REP_CONNECTION_REFUSED: u8 = 5;
pub(crate) const REP_ATYP_NOT_SUPPORTED: u8 = 8;

/// Read the client's method greeting and answer with no-auth.
pub(crate) async fn greet(sock: &mut TcpStream) -> io::Result<()> {
    let mut hdr = [0u8; 2];
    sock.read_exact(&mut hdr).await?;
    if hdr[0] != VER {
        return Err(io::Error::other("not socks5"));
    }
    let mut methods = vec![0u8; hdr[1] as usize];
    sock.read_exact(&mut methods).await?;
    sock.write_all(&[VER, 0]).await
}

/// Read a CONNECT request, returning its host and port. Replies to the client
/// and fails for anything but CONNECT with a known address type.
pub(crate) async fn read_connect(sock: &mut TcpStream) -> io::Result<(String, u16)> {
    let mut req = [0u8; 4];
    sock.read_exact(&mut req).await?;
    if req[0] != VER {
        return Err(io::Error::other("bad socks version"));
    }
    if req[1] != CMD_CONNECT {
        reply(sock, REP_CMD_NOT_SUPPORTED).await?;
        return Err(io::Error::other("only CONNECT is supported"));
    }
    let host = match req[3] {
        ATYP_V4 => {
            let mut a = [0u8; 4];
            sock.read_exact(&mut a).await?;
            IpAddr::from(a).to_string()
        }
        ATYP_V6 => {
            let mut a = [0u8; 16];
            sock.read_exact(&mut a).await?;
            IpAddr::from(a).to_string()
        }
        ATYP_DOMAIN => {
            let mut len = [0u8; 1];
            sock.read_exact(&mut len).await?;
            let mut d = vec![0u8; len[0] as usize];
            sock.read_exact(&mut d).await?;
            String::from_utf8(d).map_err(|_| io::Error::other("non-utf8 host"))?
        }
        _ => {
            reply(sock, REP_ATYP_NOT_SUPPORTED).await?;
            return Err(io::Error::other("unsupported atyp"));
        }
    };
    let mut p = [0u8; 2];
    sock.read_exact(&mut p).await?;
    Ok((host, u16::from_be_bytes(p)))
}

/// Send a reply with the given status code.
pub(crate) async fn reply(sock: &mut TcpStream, code: u8) -> io::Result<()> {
    sock.write_all(&[VER, code, 0, ATYP_V4, 0, 0, 0, 0, 0, 0])
        .await
}
