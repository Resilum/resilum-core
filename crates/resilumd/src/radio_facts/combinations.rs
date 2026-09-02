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
pub struct Combination {
    pub each_limit_allows: Vec<Vec<Role>>,
    pub interfaces_at_once: u32,
    pub channels_at_once: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhatTheRadioAllows {
    pub can_host_at_all: bool,
    pub while_on_a_router: HostsWhileOnARouter,
}

pub fn read_from(combinations: &[Combination]) -> WhatTheRadioAllows {
    let hosting: Vec<&Combination> = combinations
        .iter()
        .filter(|comb| allows(comb, Role::Hosting))
        .collect();
    let keeps_the_router = hosting.iter().any(|comb| {
        allows(comb, Role::OnARouter) && comb.interfaces_at_once >= 2 && comb.channels_at_once > 1
    });
    WhatTheRadioAllows {
        can_host_at_all: !hosting.is_empty(),
        while_on_a_router: if keeps_the_router {
            HostsWhileOnARouter::Confirmed
        } else {
            HostsWhileOnARouter::Refused
        },
    }
}

fn allows(combination: &Combination, wanted: Role) -> bool {
    combination
        .each_limit_allows
        .iter()
        .any(|limit| limit.contains(&wanted))
}

#[cfg(test)]
mod tests;
