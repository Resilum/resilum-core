use std::collections::HashMap;

use super::link::PeerLink;
use super::radio::ConnectionId;
use super::spec;

pub type PeerId = [u8; spec::IDENTITY_LEN];

pub struct Links {
    by_conn: HashMap<ConnectionId, PeerLink>,
}

pub enum Kept {
    Yes,
    AlreadyReachedThatWay(ConnectionId),
}

impl Default for Links {
    fn default() -> Self {
        Self::new()
    }
}

impl Links {
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_conn: HashMap::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_conn.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_conn.is_empty()
    }

    #[must_use]
    pub fn room_for_one_more(&self) -> bool {
        self.by_conn.len() < spec::PEERS_AT_ONCE
    }

    pub fn keep_unless_already_reached(&mut self, link: PeerLink) -> Kept {
        if let Some(older) = self.conn_to(link.peer) {
            return Kept::AlreadyReachedThatWay(older);
        }
        self.by_conn.insert(link.conn, link);
        Kept::Yes
    }

    pub fn hand_over(&self, conn: ConnectionId, fragment: Vec<u8>) {
        if let Some(link) = self.by_conn.get(&conn) {
            link.hand_over(fragment);
        }
    }

    pub fn part(&mut self, conn: ConnectionId) -> Option<PeerId> {
        self.by_conn.remove(&conn).map(|link| link.peer)
    }

    #[must_use]
    pub fn holds(&self, peer: PeerId) -> bool {
        self.conn_to(peer).is_some()
    }

    fn conn_to(&self, peer: PeerId) -> Option<ConnectionId> {
        self.by_conn
            .values()
            .find(|link| link.peer == peer)
            .map(|link| link.conn)
    }
}
