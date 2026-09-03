mod control;

use std::path::Path;
use std::time::{Duration, Instant};

use resilum_core::WifiGroup;

use super::{Raised, RaisesAGroup};
use control::Control;

const WHERE_IT_LISTENS: &str = "/run/wpa_supplicant";
const THE_2GHZ_CHANNEL_WE_HOST_ON: &str = "2437";
const AS_A_GROUP_OWNER: &str = "3";
const KEPT_AS_A_PERSISTENT_GROUP: &str = "2";
const THE_GROUP_APPEARS_WITHIN: Duration = Duration::from_secs(10);
const BETWEEN_LOOKS: Duration = Duration::from_millis(250);

pub struct WpaSupplicant;

pub fn if_it_can_own_a_p2p_group() -> Option<WpaSupplicant> {
    (!sockets_starting_with("p2p-dev-").is_empty()).then_some(WpaSupplicant)
}

impl RaisesAGroup for WpaSupplicant {
    fn raise(&self, group: &WifiGroup, interface: &str) -> Result<Raised, String> {
        let device_name = format!("p2p-dev-{interface}");
        let device = Control::to(&device_name)?;
        let network = device.asked("ADD_NETWORK")?.trim().to_owned();
        match owning_the_group(&device, group, interface, &network) {
            Ok(carried_on) => Ok(Raised {
                already_addressed: false,
                lower: lowering(device_name, network, carried_on.clone()),
                carried_on,
            }),
            Err(refused) => {
                forget(&device, &network);
                Err(refused)
            }
        }
    }
}

fn owning_the_group(
    device: &Control,
    group: &WifiGroup,
    interface: &str,
    network: &str,
) -> Result<String, String> {
    for (key, value) in describing(group) {
        device.told(&format!("SET_NETWORK {network} {key} {value}"))?;
    }
    let prefix = format!("p2p-{interface}-");
    let before = sockets_starting_with(&prefix);
    device.told(&format!(
        "P2P_GROUP_ADD persistent={network} freq={THE_2GHZ_CHANNEL_WE_HOST_ON}"
    ))?;
    whichever_socket_is_new(&prefix, &before)
}

fn describing(group: &WifiGroup) -> Vec<(&'static str, String)> {
    vec![
        ("ssid", format!("\"{}\"", group.ssid)),
        ("psk", format!("\"{}\"", group.passphrase)),
        ("mode", String::from(AS_A_GROUP_OWNER)),
        ("disabled", String::from(KEPT_AS_A_PERSISTENT_GROUP)),
        ("proto", String::from("RSN")),
        ("key_mgmt", String::from("WPA-PSK")),
        ("pairwise", String::from("CCMP")),
        ("group", String::from("CCMP")),
    ]
}

fn whichever_socket_is_new(prefix: &str, before: &[String]) -> Result<String, String> {
    let give_up_at = Instant::now() + THE_GROUP_APPEARS_WITHIN;
    while Instant::now() < give_up_at {
        if let Some(fresh) = the_one_not_there_before(sockets_starting_with(prefix), before) {
            return Ok(fresh);
        }
        std::thread::sleep(BETWEEN_LOOKS);
    }
    Err(String::from("wpa_supplicant raised no group interface"))
}

fn the_one_not_there_before(now: Vec<String>, before: &[String]) -> Option<String> {
    let mut fresh: Vec<String> = now
        .into_iter()
        .filter(|name| !before.contains(name))
        .collect();
    fresh.sort_unstable();
    fresh.pop()
}

fn sockets_starting_with(prefix: &str) -> Vec<String> {
    named_under(Path::new(WHERE_IT_LISTENS), prefix)
}

fn named_under(directory: &Path, prefix: &str) -> Vec<String> {
    let Ok(entries) = directory.read_dir() else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(prefix))
        .collect()
}

#[cfg(test)]
mod tests;

fn lowering(device_name: String, network: String, carried_on: String) -> super::Lowering {
    Box::new(move || {
        let Ok(device) = Control::to(&device_name) else {
            return;
        };
        if let Err(error) = device.told(&format!("P2P_GROUP_REMOVE {carried_on}")) {
            tracing::warn!(%error, "wpa_supplicant kept the group up");
        }
        forget(&device, &network);
    })
}

fn forget(device: &Control, network: &str) {
    if let Err(error) = device.told(&format!("REMOVE_NETWORK {network}")) {
        tracing::warn!(%error, "wpa_supplicant kept the group network");
    }
}
