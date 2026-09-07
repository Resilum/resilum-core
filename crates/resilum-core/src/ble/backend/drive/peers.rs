use std::collections::HashMap;

use crate::ble::radio::{ConnectionId, PeerAddress, Role};

#[derive(Default)]
pub(super) struct Peers {
    by_conn: HashMap<ConnectionId, (PeerAddress, Role)>,
    by_address: HashMap<PeerAddress, ConnectionId>,
    last: u64,
}

impl Peers {
    pub(super) fn joined(&mut self, address: PeerAddress, role: Role) -> ConnectionId {
        if let Some(known) = self.by_address.get(&address) {
            return *known;
        }
        self.last += 1;
        let conn = ConnectionId(self.last);
        self.by_address.insert(address.clone(), conn);
        self.by_conn.insert(conn, (address, role));
        conn
    }

    pub(super) fn conn_at(&self, address: &PeerAddress) -> Option<ConnectionId> {
        self.by_address.get(address).copied()
    }

    pub(super) fn address_of(&self, conn: ConnectionId) -> Option<PeerAddress> {
        self.by_conn.get(&conn).map(|(address, _)| address.clone())
    }

    pub(super) fn role_on(&self, conn: ConnectionId) -> Option<Role> {
        self.by_conn.get(&conn).map(|(_, role)| *role)
    }

    pub(super) fn parted(&mut self, conn: ConnectionId) -> Option<PeerAddress> {
        let (address, _) = self.by_conn.remove(&conn)?;
        self.by_address.remove(&address);
        Some(address)
    }
}

#[cfg(test)]
mod tests {
    use super::Peers;
    use crate::ble::radio::{PeerAddress, Role};

    fn at(address: &str) -> PeerAddress {
        PeerAddress(address.to_owned())
    }

    #[test]
    fn each_peer_gets_its_own_connection() {
        let mut peers = Peers::default();

        let first = peers.joined(at("aa"), Role::Central);
        let second = peers.joined(at("bb"), Role::Peripheral);

        assert_ne!(first, second);
        assert_eq!(peers.role_on(first), Some(Role::Central));
        assert_eq!(peers.role_on(second), Some(Role::Peripheral));
    }

    #[test]
    fn a_peer_reported_twice_keeps_the_connection_it_already_had() {
        let mut peers = Peers::default();
        let first = peers.joined(at("aa"), Role::Central);

        assert_eq!(peers.joined(at("aa"), Role::Peripheral), first);
    }

    #[test]
    fn a_peer_that_left_is_forgotten_and_can_join_again() {
        let mut peers = Peers::default();
        let conn = peers.joined(at("aa"), Role::Central);

        assert_eq!(peers.parted(conn), Some(at("aa")));
        assert_eq!(peers.address_of(conn), None);
        assert_eq!(peers.conn_at(&at("aa")), None);
        assert_ne!(peers.joined(at("aa"), Role::Central), conn);
    }

    #[test]
    fn a_connection_number_is_never_handed_out_twice() {
        let mut peers = Peers::default();
        let first = peers.joined(at("aa"), Role::Central);
        peers.parted(first);

        assert_ne!(peers.joined(at("cc"), Role::Central), first);
    }
}
