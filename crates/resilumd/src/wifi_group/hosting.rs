use std::net::TcpListener;
use std::os::fd::IntoRawFd;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use resilum_core::wifi_group::GroupHandle;
use resilum_core::{Node, WifiGroup};

use super::{
    Lowering, Raised, address, dhcp, patience, radio_interface, settling, whoever_holds_the_radio,
};

#[derive(Default)]
pub struct WhetherWeHostTheGroup {
    held: Option<Held>,
    not_before: Option<Instant>,
    turned_away: u32,
}

struct Held {
    lower: Lowering,
    dhcp: dhcp::Serving,
    links: GroupHandle,
    raised_at: Instant,
}

impl WhetherWeHostTheGroup {
    pub fn follow_the_election(&mut self, node: &Node) {
        match (node.ble_hosting_the_group(), self.held.is_some()) {
            (true, false) => self.raise(node),
            (false, true) => self.stand_down(),
            (true, true) => self.give_the_radio_back_if_nobody_came(),
            (false, false) => {}
        }
    }

    fn give_the_radio_back_if_nobody_came(&mut self) {
        let Some(held) = self.held.as_ref() else {
            return;
        };
        if held.links.how_many_it_carries() > 0 {
            self.turned_away = 0;
            return;
        }
        if !patience::it_carried_nobody(0, held.raised_at.elapsed()) {
            return;
        }
        self.turned_away = self.turned_away.saturating_add(1);
        let waiting = patience::before_trying_again(self.turned_away);
        self.not_before = Some(Instant::now() + waiting);
        tracing::info!(
            ?waiting,
            "nobody joined the group; giving the radio back for now"
        );
        self.stand_down();
    }

    fn raise(&mut self, node: &Node) {
        if self.not_before.is_some_and(|when| Instant::now() < when) {
            return;
        }
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
        let Some(Held {
            lower, dhcp, links, ..
        }) = self.held.take()
        else {
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
            raised_at: Instant::now(),
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
    settling::settles_on(carrying, group.owner_address)?;
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
