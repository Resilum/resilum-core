mod combinations;
mod uplink;

use futures::stream::TryStreamExt;
use resilum_core::Node;
use resilum_core::ble::election::{Facts, HostsWhileOnARouter};
use wl_nl80211::Nl80211Attr;

use combinations::WhatTheRadioAllows;

#[derive(Default)]
pub struct WhatThisHostKnows {
    radio: Option<WhatTheRadioAllows>,
    told: Option<(Facts, bool)>,
}

impl WhatThisHostKnows {
    pub fn tell(&mut self, node: &Node) {
        let radio = *self.radio.get_or_insert_with(what_the_radio_allows);
        let way_out = uplink::whichever_interface_reaches_the_world();
        let facts = Facts {
            has_an_uplink: way_out.is_some(),
            p2p_and_sta_at_once: whether_hosting_keeps_the_uplink(
                radio,
                way_out.as_deref(),
                the_radio_we_would_host_on(node).as_deref(),
            ),
            ..Facts::default()
        };
        let telling = (facts, radio.can_host_at_all);
        if self.told == Some(telling) {
            return;
        }
        node.ble_facts_reported(facts, radio.can_host_at_all);
        self.told = Some(telling);
    }
}

fn whether_hosting_keeps_the_uplink(
    radio: WhatTheRadioAllows,
    way_out: Option<&str>,
    we_would_host_on: Option<&str>,
) -> HostsWhileOnARouter {
    match (way_out, we_would_host_on) {
        (Some(out), Some(hosting)) if out != hosting => HostsWhileOnARouter::Confirmed,
        _ => radio.while_on_a_router,
    }
}

fn the_radio_we_would_host_on(node: &Node) -> Option<String> {
    let named = node.config().wifi_group.as_ref()?.interface.clone();
    crate::wifi_group::radio_interface::the_one_to_host_on(named.as_deref()).ok()
}

fn what_the_radio_allows() -> WhatTheRadioAllows {
    match asked_of_the_kernel() {
        Ok(allows) => allows,
        Err(error) => {
            tracing::debug!(%error, "the radio would not say what it allows");
            WhatTheRadioAllows {
                can_host_at_all: false,
                while_on_a_router: resilum_core::ble::election::HostsWhileOnARouter::NoOneCanSay,
            }
        }
    }
}

fn asked_of_the_kernel() -> Result<WhatTheRadioAllows, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .map_err(|e| format!("no runtime to ask over: {e}"))?;
    runtime.block_on(every_combination())
}

async fn every_combination() -> Result<WhatTheRadioAllows, String> {
    let (connection, handle, _) =
        wl_nl80211::new_connection().map_err(|e| format!("no netlink socket: {e}"))?;
    tokio::spawn(connection);
    let mut physics = handle.wireless_physic().get().execute().await;
    let mut found = Vec::new();
    while let Some(physic) = physics
        .try_next()
        .await
        .map_err(|e| format!("the kernel named no radio: {e}"))?
    {
        for attribute in physic.payload.attributes {
            if let Nl80211Attr::InterfaceCombination(said) = attribute {
                found.extend(combinations::heard_from_the_kernel(&said));
            }
        }
    }
    Ok(combinations::read_from(&found))
}

#[cfg(test)]
mod tests;
