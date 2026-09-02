mod combinations;
mod uplink;

use futures::stream::TryStreamExt;
use resilum_core::Node;
use resilum_core::ble::election::Facts;
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
        let facts = Facts {
            has_an_uplink: uplink::this_host_can_reach_the_world(),
            p2p_and_sta_at_once: radio.while_on_a_router,
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
