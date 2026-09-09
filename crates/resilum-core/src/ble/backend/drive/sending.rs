use blew::central::WriteType;

use super::Held;
use crate::ble::radio::{Outbound, Role};

pub(super) async fn put_on_the_air(held: &Held, piece: Outbound) {
    let Some((address, role)) = held
        .told
        .peers
        .address_of(piece.conn)
        .zip(held.told.peers.role_on(piece.conn))
    else {
        return;
    };
    let want = uuid::Uuid::from_u128(piece.characteristic);
    let device = blew::DeviceId::from(address.0);
    let sent = match role {
        Role::Central => {
            held.central
                .write_characteristic(&device, want, piece.value, how(piece.acknowledged))
                .await
        }
        Role::Peripheral => {
            held.peripheral
                .notify_characteristic(&device, want, piece.value)
                .await
        }
    };
    if let Err(error) = sent {
        tracing::warn!(%error, ?role, "a fragment never reached the peer");
    }
}

fn how(acknowledged: bool) -> WriteType {
    if acknowledged {
        WriteType::WithResponse
    } else {
        WriteType::WithoutResponse
    }
}
