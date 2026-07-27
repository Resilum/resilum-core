//! ICMP carrier construction for the in-process covert bridge.

use std::net::IpAddr;
use std::sync::Arc;

use leviculum_std::api::{Identity, Node as LevNode};
use leviculum_std::interfaces::ByteChannelHandle;

use crate::covert::icmp::client::IcmpClient;
use crate::covert::icmp::id::tunnel_id;

/// Build an ICMP carrier for `addr` (an IP) and bridge it in-process.
pub(super) fn attach(
    engine: &Arc<LevNode>,
    name: &str,
    addr: &str,
    server_pubkey: &[u8],
    mtu: usize,
) -> Result<ByteChannelHandle, String> {
    let addr: IpAddr = addr
        .parse()
        .map_err(|_| format!("covert endpoint address is not an IP: {addr}"))?;
    let server = Identity::from_public_key_bytes(server_pubkey)
        .map_err(|e| format!("covert server identity: {e:?}"))?;
    let ident = tunnel_id(&server.public_key_bytes());
    let carrier =
        IcmpClient::with_mtu(addr, ident, mtu).map_err(|e| format!("open icmp socket: {e}"))?;
    super::bridge(engine, name, carrier, server, super::random_session_id())
}
