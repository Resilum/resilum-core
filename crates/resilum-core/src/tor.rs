use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use arti_client::config::CfgPath;
use arti_client::{BootstrapBehavior, TorClient, TorClientConfig};
use tokio::net::TcpListener;

use self::socks::handle_conn;
use crate::letting_go::ItWasAlreadyThere as _;

mod socks;

pub type ArtiClient = Arc<TorClient<tor_rtcompat::PreferredRuntime>>;

pub struct EmbeddedTor {
    port: u16,
    accept: resilum_tasks::Watched,
    client: ArtiClient,
}

impl EmbeddedTor {
    pub async fn start_without_waiting_for_the_directory(
        a_dir_of_ours_to_keep_state_in: Option<&Path>,
        conns: Arc<resilum_tasks::Nursery>,
    ) -> io::Result<Self> {
        the_tls_arti_panics_without();
        let state_root = a_dir_of_ours_to_keep_state_in;
        let config =
            build_config(state_root).map_err(|e| io::Error::other(format!("arti config: {e}")))?;
        let client: ArtiClient = TorClient::builder()
            .config(config)
            .bootstrap_behavior(BootstrapBehavior::OnDemand)
            .create_unbootstrapped()
            .map_err(|e| io::Error::other(format!("arti client: {e}")))?;

        let warm = Arc::clone(&client);
        conns.keep("tor: bootstrapping the client", async move {
            if let Err(e) = warm.bootstrap().await {
                tracing::warn!(error = %e, "arti bootstrap failed; retried on first use");
            }
        });

        let listener =
            TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
        let port = listener.local_addr()?.port();

        let for_task = Arc::clone(&client);
        let taking = Arc::clone(&conns);
        let accept = resilum_tasks::watch("tor: taking socks connections", async move {
            accept_loop(listener, for_task, taking).await;
        });

        Ok(Self {
            port,
            accept,
            client,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn is_bootstrapped(&self) -> bool {
        self.client.bootstrap_status().ready_for_traffic()
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
    trust_the_sandboxed_dir_we_handed_it(&mut builder);
    builder.build()
}

fn trust_the_sandboxed_dir_we_handed_it(builder: &mut arti_client::config::TorClientConfigBuilder) {
    builder.storage().permissions().dangerously_trust_everyone();
}

fn the_tls_arti_panics_without() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .it_was_already_there();
}

async fn accept_loop(
    listener: TcpListener,
    client: ArtiClient,
    conns: Arc<resilum_tasks::Nursery>,
) {
    loop {
        let Ok((conn, _peer)) = listener.accept().await else {
            return;
        };
        let client = Arc::clone(&client);
        conns.keep("tor: one socks connection", async move {
            if let Err(e) = handle_conn(conn, client).await {
                tracing::debug!(error = %e, "arti socks conn ended");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn spawn_returns_without_waiting_for_the_directory() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        let tor = tokio::time::timeout(
            Duration::from_secs(10),
            EmbeddedTor::start_without_waiting_for_the_directory(Some(dir.path()), Arc::default()),
        )
        .await
        .expect("spawn must not wait for the directory")
        .expect("spawn must succeed");
        assert!(tor.port() != 0, "the SOCKS port has to be known at once");
    }
}
