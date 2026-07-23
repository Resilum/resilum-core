use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use super::ArtiClient;

const SOCKS_VER: u8 = 5;
const CMD_CONNECT: u8 = 1;
const REP_OK: u8 = 0;
const REP_UNREACHABLE: u8 = 5;
const REP_CMD_NOT_SUPPORTED: u8 = 7;
const REP_ATYP_NOT_SUPPORTED: u8 = 8;
const ATYP_V4: u8 = 1;
const ATYP_DOMAIN: u8 = 3;
const ATYP_V6: u8 = 4;

pub async fn handle_conn(mut client_sock: TcpStream, tor: ArtiClient) -> io::Result<()> {
    greet(&mut client_sock).await?;
    let (host, port) = read_connect(&mut client_sock).await?;
    match tor.connect((host.as_str(), port)).await {
        Ok(stream) => {
            reply(&mut client_sock, REP_OK).await?;
            let mut tor_stream = stream.compat();
            let _ = tokio::io::copy_bidirectional(&mut client_sock, &mut tor_stream).await;
            Ok(())
        }
        Err(e) => {
            reply(&mut client_sock, REP_UNREACHABLE).await?;
            Err(io::Error::other(format!("tor connect: {e}")))
        }
    }
}

async fn greet(sock: &mut TcpStream) -> io::Result<()> {
    let mut hdr = [0u8; 2];
    sock.read_exact(&mut hdr).await?;
    if hdr[0] != SOCKS_VER {
        return Err(io::Error::other("not socks5"));
    }
    let mut methods = vec![0u8; hdr[1] as usize];
    sock.read_exact(&mut methods).await?;
    sock.write_all(&[SOCKS_VER, 0]).await?; // NoAuth
    Ok(())
}

async fn read_connect(sock: &mut TcpStream) -> io::Result<(String, u16)> {
    let mut req = [0u8; 4];
    sock.read_exact(&mut req).await?;
    if req[0] != SOCKS_VER {
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
            std::net::IpAddr::from(a).to_string()
        }
        ATYP_V6 => {
            let mut a = [0u8; 16];
            sock.read_exact(&mut a).await?;
            std::net::IpAddr::from(a).to_string()
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

async fn reply(sock: &mut TcpStream, code: u8) -> io::Result<()> {
    sock.write_all(&[SOCKS_VER, code, 0, ATYP_V4, 0, 0, 0, 0, 0, 0])
        .await
}
