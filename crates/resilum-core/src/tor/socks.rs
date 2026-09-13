use std::io;

use tokio::net::TcpStream;
use tokio_util::compat::FuturesAsyncReadCompatExt as _;

use super::ArtiClient;
use crate::socks5_tcp::{self, REP_CONNECTION_REFUSED, REP_OK};

pub async fn handle_conn(mut client_sock: TcpStream, tor: ArtiClient) -> io::Result<()> {
    socks5_tcp::greet(&mut client_sock).await?;
    let (host, port) = socks5_tcp::read_connect(&mut client_sock).await?;
    match tor.connect((host.as_str(), port)).await {
        Ok(stream) => {
            socks5_tcp::reply(&mut client_sock, REP_OK).await?;
            let mut tor_stream = stream.compat();
            crate::pump::both_ways(&mut client_sock, &mut tor_stream, "a flow through tor").await;
            Ok(())
        }
        Err(e) => {
            socks5_tcp::reply(&mut client_sock, REP_CONNECTION_REFUSED).await?;
            Err(io::Error::other(format!("tor connect: {e}")))
        }
    }
}
