//! One inbound link, forwarded to its service's TCP endpoint.

use std::net::SocketAddr;

use leviculum_std::api::LinkHandle;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, lookup_host};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::egress::socks5;
use crate::link::LinkMsg;

pub(super) async fn session_external(
    handle: LinkHandle,
    from_link: UnboundedReceiver<LinkMsg>,
    target: String,
) {
    let tcp = match TcpStream::connect(&target).await {
        Ok(tcp) => tcp,
        Err(e) => {
            tracing::warn!(target = %target, error = %e, "egress connect failed");
            let _ = close(handle).await;
            return;
        }
    };
    pump_link(handle, from_link, tcp).await;
}

pub(super) async fn session_embedded(
    handle: LinkHandle,
    mut from_link: UnboundedReceiver<LinkMsg>,
    allow_private: bool,
) {
    let req = match socks5::handshake(&mut from_link).await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(error = %e, "socks handshake failed");
            let _ = close(handle).await;
            return;
        }
    };
    if handle.send(&socks5::AUTH_NO_AUTH).await.is_err() {
        return;
    }
    // Resolved addresses, not the requested name: a filter on the name is
    // bypassed by any name that resolves inward.
    let addrs: Vec<SocketAddr> = match lookup_host((req.host.as_str(), req.port)).await {
        Ok(a) => a.collect(),
        Err(e) => {
            tracing::debug!(host = %req.host, port = req.port, error = %e, "resolve failed");
            let _ = handle.send(&socks5::REPLY_HOST_UNREACHABLE).await;
            let _ = close(handle).await;
            return;
        }
    };
    if !super::policy::dialable(&addrs, allow_private) {
        tracing::warn!(host = %req.host, port = req.port, "egress target refused by policy");
        let _ = handle.send(&socks5::REPLY_NOT_ALLOWED).await;
        let _ = close(handle).await;
        return;
    }
    let mut tcp = match TcpStream::connect(&addrs[..]).await {
        Ok(tcp) => tcp,
        Err(e) => {
            tracing::debug!(host = %req.host, port = req.port, error = %e, "upstream unreachable");
            let _ = handle.send(&socks5::REPLY_HOST_UNREACHABLE).await;
            let _ = close(handle).await;
            return;
        }
    };
    if handle.send(&socks5::REPLY_OK).await.is_err() {
        return;
    }
    if !req.leftover.is_empty() && tcp.write_all(&req.leftover).await.is_err() {
        let _ = close(handle).await;
        return;
    }
    pump_link(handle, from_link, tcp).await;
}

async fn pump_link(handle: LinkHandle, from_link: UnboundedReceiver<LinkMsg>, tcp: TcpStream) {
    crate::egress::relay::relay(&handle, from_link, tcp).await;
    let _ = close(handle).await;
}

async fn close(mut handle: LinkHandle) {
    let _ = handle.close().await;
}
