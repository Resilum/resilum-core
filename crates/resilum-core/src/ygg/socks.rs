//! Loopback SOCKS5 proxy for the initiate side. leviculum dials a ygg peer by
//! CONNECTing here (the yggdrasil discovery service's `socks_proxy` points at
//! it); we open the connection over the ygg stack. Yggdrasil peers are IPv6, so
//! only an IP CONNECT target is accepted.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio_smoltcp::Net;

use crate::socks5_tcp::{self, REP_ATYP_NOT_SUPPORTED, REP_CONNECTION_REFUSED, REP_OK};

pub(super) async fn serve(net: Arc<Net>, port: u16) {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(port, error = %e, "ygg socks bind failed");
            return;
        }
    };
    while let Ok((client, _)) = listener.accept().await {
        let net = net.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(client, net).await {
                tracing::debug!(error = %e, "ygg socks conn ended");
            }
        });
    }
}

async fn handle_conn(mut client: TcpStream, net: Arc<Net>) -> io::Result<()> {
    socks5_tcp::greet(&mut client).await?;
    let (host, port) = socks5_tcp::read_connect(&mut client).await?;
    let Ok(ip) = host.parse::<IpAddr>() else {
        socks5_tcp::reply(&mut client, REP_ATYP_NOT_SUPPORTED).await?;
        return Err(io::Error::other("ygg expects an IP CONNECT target"));
    };
    match net.tcp_connect(SocketAddr::new(ip, port)).await {
        Ok(mut upstream) => {
            socks5_tcp::reply(&mut client, REP_OK).await?;
            let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
            Ok(())
        }
        Err(e) => {
            socks5_tcp::reply(&mut client, REP_CONNECTION_REFUSED).await?;
            Err(io::Error::other(format!("ygg connect: {e}")))
        }
    }
}
