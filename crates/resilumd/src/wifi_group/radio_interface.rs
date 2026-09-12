use std::path::Path;

const NETWORK_DEVICES: &str = "/sys/class/net";

pub fn the_one_to_host_on(named: Option<&str>) -> Result<String, String> {
    chosen_from(Path::new(NETWORK_DEVICES), named)
}

fn chosen_from(devices: &Path, named: Option<&str>) -> Result<String, String> {
    if let Some(named) = named {
        return Ok(named.to_owned());
    }
    match wireless_ones_under(devices).as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(String::from("this host has no wireless interface")),
        many => Err(format!(
            "this host has {} wireless interfaces; name one in the config",
            many.len()
        )),
    }
}

fn wireless_ones_under(devices: &Path) -> Vec<String> {
    let Ok(entries) = resilum_store::list(devices) else {
        return Vec::new();
    };
    let mut wireless: Vec<String> = entries
        .iter()
        .filter(|entry| entry.join("phy80211").exists())
        .filter_map(|entry| entry.file_name()?.to_str().map(str::to_owned))
        .collect();
    wireless.sort_unstable();
    wireless
}

#[cfg(test)]
mod tests;
