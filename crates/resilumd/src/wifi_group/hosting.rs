use std::net::TcpListener;
use std::os::fd::IntoRawFd;
use std::time::{SystemTime, UNIX_EPOCH};

use resilum_core::wifi_group::GroupHandle;
use resilum_core::{Node, WifiGroup};

use super::{Lowering, Raised, address, dhcp, radio_interface, whoever_holds_the_radio};

#[derive(Default)]
pub struct WhetherWeHostTheGroup {
    held: Option<Held>,
}

struct Held {
    lower: Lowering,
    dhcp: dhcp::Serving,
    links: GroupHandle,
}

impl WhetherWeHostTheGroup {
    pub fn follow_the_election(&mut self, node: &Node) {
        match (node.ble_hosting_the_group(), self.held.is_some()) {
            (true, false) => self.raise(node),
            (false, true) => self.stand_down(),
            _ => {}
        }
    }

    fn raise(&mut self, node: &Node) {
        let Some(group) = node.config().wifi_group.clone() else {
            return;
        };
        match raised(node, &group) {
            Ok(held) => {
                tracing::info!(ssid = %group.ssid, "raised the wi-fi group this node won");
                self.held = Some(held);
            }
            Err(error) => tracing::warn!(%error, "the wi-fi group would not come up"),
        }
    }

    fn stand_down(&mut self) {
        let Some(Held { lower, dhcp, links }) = self.held.take() else {
            return;
        };
        links.detach();
        dhcp.stop();
        lower();
        tracing::info!("stood down from hosting the wi-fi group");
    }
}

impl Drop for WhetherWeHostTheGroup {
    fn drop(&mut self) {
        self.stand_down();
    }
}

fn raised(node: &Node, group: &WifiGroup) -> Result<Held, String> {
    let interface = radio_interface::the_one_to_host_on(group.interface.as_deref())?;
    let radio = whoever_holds_the_radio()
        .ok_or_else(|| String::from("no daemon on this host holds the radio"))?;
    let raised = radio.raise(group, &interface)?;
    match carried_over(node, group, &raised) {
        Ok((serving, links)) => Ok(Held {
            lower: raised.lower,
            dhcp: serving,
            links,
        }),
        Err(refused) => {
            (raised.lower)();
            Err(refused)
        }
    }
}

fn carried_over(
    node: &Node,
    group: &WifiGroup,
    raised: &Raised,
) -> Result<(dhcp::Serving, GroupHandle), String> {
    let carrying = &raised.carried_on;
    if !raised.already_addressed {
        address::put_on(carrying, group.owner_address)?;
    }
    let serving = dhcp::on(carrying, group.owner_address, seconds_since_the_epoch)
        .map_err(|e| format!("no dhcp on {carrying}: {e}"))?;
    let listening = TcpListener::bind((group.owner_address, group.port))
        .map_err(|e| format!("nothing listening on the group: {e}"))?;
    let links = node
        .wifi_group_hosting(listening.into_raw_fd())
        .map_err(|e| e.to_string())?;
    Ok((serving, links))
}

fn seconds_since_the_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
