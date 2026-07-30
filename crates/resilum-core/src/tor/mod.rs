//! In-process Tor: bootstraps Arti and exposes a local SOCKS5 listener.

mod socks;

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use arti_client::config::CfgPath;
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
    /// Bootstrap Arti. `state_root`, when set, roots its cache/state there — a
    /// sandboxed host lacks a writable OS-default dir for them.
    pub async fn spawn(state_root: Option<&Path>) -> io::Result<Self> {
        let config =
            build_config(state_root).map_err(|e| io::Error::other(format!("arti config: {e}")))?;
        let client: ArtiClient = TorClient::create_bootstrapped(config)
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

fn build_config(
    state_root: Option<&Path>,
) -> Result<TorClientConfig, arti_client::config::ConfigBuildError> {
    let Some(root) = state_root else {
        return Ok(TorClientConfig::default());
    };
    let mut builder = TorClientConfig::builder();
    builder
        .storage()
        .cache_dir(CfgPath::new(
            root.join("tor/cache").to_string_lossy().into_owned(),
        ))
        .state_dir(CfgPath::new(
            root.join("tor/state").to_string_lossy().into_owned(),
        ));
    // Arti's default fs-permission checks reject a sandboxed app dir; trust ours.
    builder.storage().permissions().dangerously_trust_everyone();
    builder.build()
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
