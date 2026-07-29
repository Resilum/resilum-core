use std::io;

use tokio::net::TcpStream;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use super::ArtiClient;
use crate::socks5_tcp::{self, REP_OK, REP_UNREACHABLE};

pub async fn handle_conn(mut client_sock: TcpStream, tor: ArtiClient) -> io::Result<()> {
    socks5_tcp::greet(&mut client_sock).await?;
    let (host, port) = socks5_tcp::read_connect(&mut client_sock).await?;
    match tor.connect((host.as_str(), port)).await {
        Ok(stream) => {
            socks5_tcp::reply(&mut client_sock, REP_OK).await?;
            let mut tor_stream = stream.compat();
            let _ = tokio::io::copy_bidirectional(&mut client_sock, &mut tor_stream).await;
            Ok(())
        }
        Err(e) => {
            socks5_tcp::reply(&mut client_sock, REP_UNREACHABLE).await?;
            Err(io::Error::other(format!("tor connect: {e}")))
        }
    }
}
