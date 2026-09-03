mod from_netlink;

use resilum_core::ble::election::HostsWhileOnARouter;

pub use from_netlink::heard_from_the_kernel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Hosting,
    OnARouter,
    Something,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limit {
    pub allows: Vec<Role>,
    pub at_most: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Combination {
    pub each_limit: Vec<Limit>,
    pub interfaces_at_once: u32,
    pub channels_at_once: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhatTheRadioAllows {
    pub can_host_at_all: bool,
    pub while_on_a_router: HostsWhileOnARouter,
}

pub fn read_from(combinations: &[Combination]) -> WhatTheRadioAllows {
    let keeps_the_router = combinations.iter().any(|comb| {
        a_station_and_a_host_both_fit(comb)
            && comb.interfaces_at_once >= 2
            && comb.channels_at_once > 1
    });
    WhatTheRadioAllows {
        can_host_at_all: combinations.iter().any(|comb| allows(comb, Role::Hosting)),
        while_on_a_router: if keeps_the_router {
            HostsWhileOnARouter::Confirmed
        } else {
            HostsWhileOnARouter::Refused
        },
    }
}

fn allows(combination: &Combination, wanted: Role) -> bool {
    combination
        .each_limit
        .iter()
        .any(|limit| limit.allows.contains(&wanted))
}

fn a_station_and_a_host_both_fit(combination: &Combination) -> bool {
    let limits = &combination.each_limit;
    limits.iter().enumerate().any(|(nth, for_the_router)| {
        for_the_router.allows.contains(&Role::OnARouter)
            && limits.iter().enumerate().any(|(other, for_hosting)| {
                for_hosting.allows.contains(&Role::Hosting)
                    && (other != nth || for_hosting.at_most >= 2)
            })
    })
}

#[cfg(test)]
mod tests;
