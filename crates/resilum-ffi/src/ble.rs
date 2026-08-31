use std::ffi::c_int;

use resilum_core::ble::election::{Facts, HostsWhileOnARouter};

pub const RESILUM_HOSTS_WHILE_ON_A_ROUTER_CONFIRMED: c_int = 1;
pub const RESILUM_HOSTS_WHILE_ON_A_ROUTER_NO_ONE_CAN_SAY: c_int = 0;
pub const RESILUM_HOSTS_WHILE_ON_A_ROUTER_REFUSED: c_int = -1;

use crate::node::ResilumNode;

/// # Safety
/// `node` must be a live handle from `resilum_node_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_ble_facts_reported(
    node: *mut ResilumNode,
    can_host_at_all: bool,
    has_an_uplink: bool,
    hosts_while_on_a_router: c_int,
    charging: bool,
    battery_percent: u8,
) -> c_int {
    crate::guard(crate::RESILUM_ERR_NULL, || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            crate::set_error(String::from("ble facts: null node"));
            return crate::RESILUM_ERR_NULL;
        };
        node.0.ble_facts_reported(
            Facts {
                has_an_uplink,
                p2p_and_sta_at_once: told(hosts_while_on_a_router),
                charging,
                battery_percent,
                neighbours_heard: 0,
            },
            can_host_at_all,
        );
        crate::RESILUM_OK
    })
}

fn told(hosts_while_on_a_router: c_int) -> HostsWhileOnARouter {
    match hosts_while_on_a_router {
        RESILUM_HOSTS_WHILE_ON_A_ROUTER_CONFIRMED => HostsWhileOnARouter::Confirmed,
        RESILUM_HOSTS_WHILE_ON_A_ROUTER_REFUSED => HostsWhileOnARouter::Refused,
        _ => HostsWhileOnARouter::NoOneCanSay,
    }
}
