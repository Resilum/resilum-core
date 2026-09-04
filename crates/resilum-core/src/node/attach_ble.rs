use std::sync::Arc;

use crate::ble::radio::Radio;
use crate::ble::spec;
use crate::error::{Error, Result};
use crate::node::Node;

impl Node {
    pub fn ble_facts_reported(&self, facts: crate::ble::election::Facts, can_host_at_all: bool) {
        self.ble_facts.report(facts, can_host_at_all);
    }

    #[must_use]
    pub fn ble_hosting_the_group(&self) -> bool {
        self.ble_hosting.is_up()
    }

    #[must_use]
    pub fn ble_someone_else_hosts_a_group(&self) -> bool {
        !self.ble_hosting.is_up() && self.ble_someone_elses_group.is_up()
    }

    #[must_use]
    pub fn ble_candidates(&self) -> Vec<crate::ble::election::Candidate> {
        self.ble_field.standing()
    }

    pub fn ble_attach_a_radio_the_caller_owns(&mut self, radio: Arc<dyn Radio>) -> Result<()> {
        let engine = self.engine().ok_or(Error::NotRunning)?;
        let identity = self.identity.clone().ok_or(Error::NotRunning)?;
        let router = self.router.clone().ok_or(Error::NotRunning)?;
        let inbox = self.inbox.clone().ok_or(Error::NotRunning)?;
        let mut ours = [0u8; spec::IDENTITY_LEN];
        ours.copy_from_slice(&engine.identity_hash()[..spec::IDENTITY_LEN]);
        crate::node::lifecycle::ble::speak_over(
            self,
            &crate::node::lifecycle::ble::Wiring {
                engine: &engine,
                identity: &identity,
                router: &router,
                inbox: &inbox,
            },
            ours,
            async move { Some(radio) },
        );
        Ok(())
    }
}
