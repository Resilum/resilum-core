use std::os::fd::RawFd;

use crate::error::{Error, Result};
use crate::node::Node;
use crate::wifi_group::{self, GroupHandle, Ours};

impl Node {
    pub fn wifi_group_joined(
        &self,
        a_socket_connected_to_the_group_owner: RawFd,
    ) -> Result<GroupHandle> {
        let _guard = self.runtime.enter();
        wifi_group::joined(
            self.over_the_group()?,
            a_socket_connected_to_the_group_owner,
        )
        .map_err(|e| Error::Engine(e.to_string()))
    }

    pub fn wifi_group_hosting(
        &self,
        a_socket_listening_where_phones_join: RawFd,
    ) -> Result<GroupHandle> {
        let _guard = self.runtime.enter();
        wifi_group::hosting(self.over_the_group()?, a_socket_listening_where_phones_join)
            .map_err(|e| Error::Engine(e.to_string()))
    }

    fn over_the_group(&self) -> Result<Ours> {
        Ok(Ours {
            engine: self.engine.clone().ok_or(Error::NotRunning)?,
            origins: self.origin_registry.clone(),
            attachments: self.attachments.clone(),
        })
    }
}
