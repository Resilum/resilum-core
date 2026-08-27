//! ICMP carrier construction for the in-process covert bridge.

use std::sync::Arc;

use leviculum_std::api::Identity;
use leviculum_std::driver::ReticulumNode;
use leviculum_std::interfaces::ByteChannelHandle;

use super::DialableAddress;
use crate::covert::icmp::client::IcmpClient;
use crate::covert::icmp::id::tunnel_id;
use crate::covert::icmp::server::IcmpServer;
use crate::covert::runner;

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
    let session = super::random_session_id();
    super::bridge(engine, name, move |uplink, decoded| {
        let _ = runner::run_client(carrier, server, session, uplink, move |bytes| {
            decoded.hand_to_leviculum(bytes);
        });
    })
}

pub(super) fn listen(
    engine: &Arc<ReticulumNode>,
    name: &str,
    identity: Identity,
    mtu: usize,
) -> Result<ByteChannelHandle, String> {
    let ident = tunnel_id(&identity.public_key_bytes());
    let carrier =
        IcmpServer::with_mtu(ident, mtu).map_err(|e| format!("open icmp server socket: {e}"))?;
    super::bridge(engine, name, move |uplink, decoded| {
        let _ = runner::run_server(carrier, identity, uplink, move |_session, bytes| {
            decoded.hand_to_leviculum(bytes);
        });
    })
}
