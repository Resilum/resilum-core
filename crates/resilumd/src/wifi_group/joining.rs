use std::net::TcpStream;
use std::os::fd::IntoRawFd;
use std::time::{Duration, Instant};

use resilum_core::wifi_group::GroupHandle;
use resilum_core::{Node, WifiGroup};

use super::{Lowering, network_manager, radio_interface, settings};

const THE_OWNER_ANSWERS_WITHIN: Duration = Duration::from_secs(30);
const BETWEEN_KNOCKS: Duration = Duration::from_millis(500);
const A_KNOCK_WAITS: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct WhetherWeJoinTheGroup {
    held: Option<Held>,
}

struct Held {
    leave: Lowering,
    links: GroupHandle,
}

impl WhetherWeJoinTheGroup {
    pub fn follow_the_election(&mut self, node: &Node) {
        match (worth_entering(node), self.held.is_some()) {
            (true, false) => self.join(node),
            (false, true) => self.leave(),
            _ => {}
        }
    }

    fn join(&mut self, node: &Node) {
        let Some(group) = node.config().wifi_group.clone() else {
            return;
        };
        match joined(node, &group) {
            Ok(held) => {
                tracing::info!(ssid = %group.ssid, "joined the wi-fi group a neighbour raised");
                self.held = Some(held);
            }
            Err(error) => tracing::warn!(%error, "the wi-fi group would not let us in"),
        }
    }

    fn leave(&mut self) {
        let Some(Held { leave, links }) = self.held.take() else {
            return;
        };
        links.detach();
        leave();
        tracing::info!("left the wi-fi group");
    }
}

impl Drop for WhetherWeJoinTheGroup {
    fn drop(&mut self) {
        self.leave();
    }
}

fn worth_entering(node: &Node) -> bool {
    node.ble_someone_else_hosts_a_group()
        && !crate::radio_facts::taking_part_would_cost_the_way_out(node)
}

fn joined(node: &Node, group: &WifiGroup) -> Result<Held, String> {
    let interface = radio_interface::the_one_to_host_on(group.interface.as_deref())?;
    let leave = network_manager::activate(
        settings::a_group_someone_else_owns(group, &interface),
        &interface,
        "join the group",
    )?;
    let owner = match knocked_until_the_owner_answers(group) {
        Ok(owner) => owner,
        Err(refused) => {
            leave();
            return Err(refused);
        }
    };
    let links = node
        .wifi_group_joined(owner.into_raw_fd())
        .map_err(|e| e.to_string())?;
    Ok(Held { leave, links })
}

fn knocked_until_the_owner_answers(group: &WifiGroup) -> Result<TcpStream, String> {
    let owner = (group.owner_address, group.port).into();
    let give_up_at = Instant::now() + THE_OWNER_ANSWERS_WITHIN;
    let mut refused = String::new();
    while Instant::now() < give_up_at {
        match TcpStream::connect_timeout(&owner, A_KNOCK_WAITS) {
            Ok(open) => return Ok(open),
            Err(e) => refused = e.to_string(),
        }
        std::thread::sleep(BETWEEN_KNOCKS);
    }
    Err(format!("{owner} never answered: {refused}"))
}

#[cfg(test)]
mod tests;
