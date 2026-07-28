//! UDP data plane. DNS is answered locally by FakeDNS so names never leak to a
//! real resolver; other datagrams are dropped for now (upstream UDP relay is not
//! yet wired).

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use netstack_smoltcp::UdpSocket;

use super::fakedns::FakeDns;

const DNS_PORT: u16 = 53;

pub(super) async fn serve(socket: UdpSocket, fakedns: Arc<FakeDns>) {
    let (mut rx, mut tx) = socket.split();
    while let Some((payload, local, remote)) = rx.next().await {
        if remote.port() != DNS_PORT {
            continue;
        }
        if let Some(reply) = fakedns.answer(&payload) {
            // Reply appears to come from the resolver the app queried.
            if tx.send((reply, remote, local)).await.is_err() {
                break;
            }
        }
    }
}
