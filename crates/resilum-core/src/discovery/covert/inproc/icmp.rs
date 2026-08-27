//! ICMP carrier construction for the in-process covert bridge.

use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;

use super::DialableAddress;
use crate::covert::icmp::client::IcmpClient;
use crate::covert::icmp::id::tunnel_id;

pub(super) fn attach(
    engine: &Arc<ReticulumNode>,
    name: &str,
    addr: &DialableAddress,
    server_pubkey: &[u8],
    mtu: usize,
) -> Result<ByteChannelHandle, String> {
    let addr = addr.ip();
    let server = Identity::from_public_key_bytes(server_pubkey)
        .map_err(|e| format!("covert server identity: {e:?}"))?;
    let ident = tunnel_id(&server.public_key_bytes());
    let carrier =
        IcmpClient::with_mtu(addr, ident, mtu).map_err(|e| format!("open icmp socket: {e}"))?;
    super::bridge(engine, name, carrier, server, super::random_session_id())
}
