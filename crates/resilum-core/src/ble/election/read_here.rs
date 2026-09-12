use super::Facts;

#[cfg(any(target_os = "linux", target_os = "android"))]
const POWER_SUPPLY: &str = "/sys/class/power_supply";

const NOTHING_HERE_RUNS_DOWN: u8 = 100;

pub enum PowerHere {
    ABattery { percent: u8, charging: bool },
    NoBatteryAtAll,
    NotOursToRead,
}

#[must_use]
pub fn what_this_host_can_answer(known: Facts) -> Facts {
    answered_with(power_here(), known)
}

fn answered_with(power: PowerHere, known: Facts) -> Facts {
    match power {
        PowerHere::ABattery { percent, charging } => Facts {
            battery_percent: percent,
            charging,
            ..known
        },
        PowerHere::NoBatteryAtAll => Facts {
            charging: true,
            battery_percent: NOTHING_HERE_RUNS_DOWN,
            ..known
        },
        PowerHere::NotOursToRead => known,
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn power_here() -> PowerHere {
    let Ok(supplies) = resilum_store::list(std::path::Path::new(POWER_SUPPLY)) else {
        return PowerHere::NotOursToRead;
    };
    supplies
        .iter()
        .find_map(|supply| a_battery_at(supply))
        .map_or(PowerHere::NoBatteryAtAll, |(percent, charging)| {
            PowerHere::ABattery { percent, charging }
        })
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn a_battery_at(at: &std::path::Path) -> Option<(u8, bool)> {
    if said(at, "type")? != "Battery" {
        return None;
    }
    let charge = said(at, "capacity")?.parse().ok()?;
    let charging = said(at, "status").is_some_and(|status| at_a_socket(&status));
    Some((charge, charging))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn said(at: &std::path::Path, about: &str) -> Option<String> {
    resilum_store::read_text(&at.join(about))
        .ok()
        .map(|raw| raw.trim().to_owned())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn at_a_socket(status: &str) -> bool {
    status != "Discharging"
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn power_here() -> PowerHere {
    PowerHere::NotOursToRead
}

#[cfg(test)]
mod tests;
