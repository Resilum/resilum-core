use std::sync::Arc;

use blew::central::Central;

use super::{ATT_HEADER, Held};
use crate::ble::radio::{PeerAddress, RadioEvent, Role};

pub(super) struct Dialled {
    address: PeerAddress,
    bytes_one_write_carries: Option<usize>,
}

pub(super) fn connect(held: &mut Held, address: PeerAddress) {
    if !held.already_dialling.insert(address.clone()) {
        return;
    }
    held.dials_in_flight
        .spawn(dial(Arc::clone(&held.central), address));
}

async fn dial(central: Arc<Central>, address: PeerAddress) -> Dialled {
    let device = blew::DeviceId::from(address.0.clone());
    if let Err(error) = central.connect(&device).await {
        tracing::debug!(%error, peer = %address.0, "the peer would not let us in");
        return Dialled {
            address,
            bytes_one_write_carries: None,
        };
    }
    let notified = uuid::Uuid::from_u128(super::super::spec::TX_NOTIFIED_BY_THE_PERIPHERAL);
    if let Err(error) = central.subscribe_characteristic(&device, notified).await {
        tracing::warn!(%error, "the peer let us in but not to its notifications");
    }
    let carried = usize::from(central.mtu(&device).await).saturating_sub(ATT_HEADER);
    Dialled {
        address,
        bytes_one_write_carries: Some(carried),
    }
}

pub(super) async fn dialled(held: &mut Held, reached: Dialled) {
    held.already_dialling.remove(&reached.address);
    let Some(carried) = reached.bytes_one_write_carries else {
        return;
    };
    let conn = held
        .told
        .peers
        .joined(reached.address.clone(), Role::Central);
    held.told.now_carries(conn, carried);
    let _ = held
        .told
        .telling
        .send(RadioEvent::Connected {
            conn,
            address: reached.address,
            role: Role::Central,
            bytes_one_write_carries: carried,
        })
        .await;
}
