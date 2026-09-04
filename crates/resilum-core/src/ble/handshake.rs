use std::collections::HashMap;

use super::links::PeerId;
use super::radio::{ConnectionId, Outbound, PeerAddress, Radio, RadioError, Role};
use super::spec;

pub struct Waiting {
    pub address: PeerAddress,
    pub role: Role,
    pub since_ms: u64,
}

#[derive(Default)]
pub struct Handshakes {
    waiting: HashMap<ConnectionId, Waiting>,
}

pub enum Told {
    NotYet,
    ThisIsTheirIdentity(PeerId, Waiting),
    NotOneOfOurs,
}

impl Handshakes {
    pub fn began(
        &mut self,
        radio: &dyn Radio,
        conn: ConnectionId,
        address: PeerAddress,
        role: Role,
        now_ms: u64,
    ) -> Result<(), RadioError> {
        self.waiting.insert(
            conn,
            Waiting {
                address,
                role,
                since_ms: now_ms,
            },
        );
        if role == Role::Central {
            radio.read(conn, spec::IDENTITY_READ_FROM_THE_PERIPHERAL)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn is_waiting(&self, conn: ConnectionId) -> bool {
        self.waiting.contains_key(&conn)
    }

    pub fn heard(
        &mut self,
        radio: &dyn Radio,
        conn: ConnectionId,
        characteristic: u128,
        value: &[u8],
        ours: PeerId,
    ) -> Told {
        let Some(waiting) = self.waiting.get(&conn) else {
            return Told::NotYet;
        };
        if characteristic != expected_from(waiting.role) {
            return Told::NotYet;
        }
        let Ok(peer) = PeerId::try_from(value) else {
            return Told::NotOneOfOurs;
        };
        if waiting.role == Role::Central && tell_them_who_we_are(radio, conn, ours).is_err() {
            return Told::NotOneOfOurs;
        }
        match self.waiting.remove(&conn) {
            Some(waiting) => Told::ThisIsTheirIdentity(peer, waiting),
            None => Told::NotYet,
        }
    }

    pub fn gave_up_by(&mut self, now_ms: u64, after_ms: u64) -> Vec<ConnectionId> {
        let stale: Vec<ConnectionId> = self
            .waiting
            .iter()
            .filter(|(_, waiting)| now_ms.saturating_sub(waiting.since_ms) > after_ms)
            .map(|(conn, _)| *conn)
            .collect();
        for conn in &stale {
            self.waiting.remove(conn);
        }
        stale
    }

    pub fn forget(&mut self, conn: ConnectionId) {
        self.waiting.remove(&conn);
    }
}

fn tell_them_who_we_are(
    radio: &dyn Radio,
    conn: ConnectionId,
    ours: PeerId,
) -> Result<(), RadioError> {
    radio
        .outbound_awaits_a_slot()
        .try_send(Outbound {
            conn,
            characteristic: Role::Central.sends_on(),
            value: ours.to_vec(),
            acknowledged: true,
        })
        .map_err(|_| RadioError::QueueFull)
}

fn expected_from(role: Role) -> u128 {
    match role {
        Role::Central => spec::IDENTITY_READ_FROM_THE_PERIPHERAL,
        Role::Peripheral => spec::RX_WRITTEN_BY_THE_CENTRAL,
    }
}

#[cfg(test)]
mod tests;
