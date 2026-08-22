use std::net::Ipv4Addr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout};

use super::Probe;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const NO_MESH_LEG: f64 = 0.0;
const GREETING_AND_CONNECT_REPLY: usize = 12;

pub async fn over_a_local_socket(target: &str, probe_targets: &[(Ipv4Addr, u16)]) -> Option<Probe> {
    for (host, port) in probe_targets {
        if let Some(probe) = one_exchange(target, *host, *port).await {
            return Some(probe);
        }
    }
    None
}

async fn one_exchange(target: &str, host: Ipv4Addr, port: u16) -> Option<Probe> {
    let exchange = async {
        let mut exit = TcpStream::connect(target).await.ok()?;
        let started = Instant::now();
        exit.write_all(&super::greeting_and_connect(host, port))
            .await
            .ok()?;
        let mut answer = [0u8; GREETING_AND_CONNECT_REPLY];
        exit.read_exact(&mut answer).await.ok()?;
        Some(Probe {
            link_rtt: NO_MESH_LEG,
            e2e: started.elapsed().as_secs_f64(),
        })
    };
    timeout(PROBE_TIMEOUT, exchange).await.ok()?
}

#[cfg(test)]
mod tests;
