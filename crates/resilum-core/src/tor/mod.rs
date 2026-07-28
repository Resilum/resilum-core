//! In-process Tor: bootstraps Arti and exposes a local SOCKS5 listener.

mod socks;

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use arti_client::{TorClient, TorClientConfig};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use socks::handle_conn;

pub type ArtiClient = Arc<TorClient<tor_rtcompat::PreferredRuntime>>;

pub struct EmbeddedTor {
    port: u16,
    accept: JoinHandle<()>,
    client: ArtiClient,
}

impl EmbeddedTor {
    pub async fn spawn() -> io::Result<Self> {
        let client: ArtiClient = TorClient::create_bootstrapped(TorClientConfig::default())
            .await
            .map_err(|e| io::Error::other(format!("arti bootstrap: {e}")))?;

        let listener =
            TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
        let port = listener.local_addr()?.port();

        let for_task = Arc::clone(&client);
        let accept = tokio::spawn(async move { accept_loop(listener, for_task).await });

        Ok(Self {
            port,
            accept,
            client,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn client(&self) -> ArtiClient {
        Arc::clone(&self.client)
    }
}

impl Drop for EmbeddedTor {
    fn drop(&mut self) {
        self.accept.abort();
    }
}

async fn accept_loop(listener: TcpListener, client: ArtiClient) {
    loop {
        let Ok((conn, _peer)) = listener.accept().await else {
            return;
        };
        let client = Arc::clone(&client);
        tokio::spawn(async move {
            if let Err(e) = handle_conn(conn, client).await {
                tracing::debug!(error = %e, "arti socks conn ended");
            }
        });
    }
}
