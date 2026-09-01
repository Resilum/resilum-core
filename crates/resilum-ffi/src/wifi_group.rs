use std::os::raw::c_int;

use resilum_core::wifi_group::GroupHandle;

use crate::node::ResilumNode;
use crate::{guard, set_error};

pub struct ResilumWifiGroup(GroupHandle);

/// # Safety
/// A live `node`, and a connected socket this call takes over.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_wifi_group_joined(
    node: *const ResilumNode,
    a_socket_connected_to_the_group_owner: c_int,
) -> *mut ResilumWifiGroup {
    attached(node, |node| {
        node.0
            .wifi_group_joined(a_socket_connected_to_the_group_owner)
    })
}

/// # Safety
/// A live `node`, and a listening socket this call takes over.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_wifi_group_hosting(
    node: *const ResilumNode,
    a_socket_listening_where_phones_join: c_int,
) -> *mut ResilumWifiGroup {
    attached(node, |node| {
        node.0
            .wifi_group_hosting(a_socket_listening_where_phones_join)
    })
}

/// # Safety
/// A handle from one of the calls above, detached at most once, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resilum_wifi_group_detach(handle: *mut ResilumWifiGroup) {
    if !handle.is_null() {
        unsafe { Box::from_raw(handle) }.0.detach();
    }
}

fn attached(
    node: *const ResilumNode,
    open: impl FnOnce(&ResilumNode) -> resilum_core::Result<GroupHandle>,
) -> *mut ResilumWifiGroup {
    guard(std::ptr::null_mut(), || {
        let Some(node) = (unsafe { node.as_ref() }) else {
            set_error("null node");
            return std::ptr::null_mut();
        };
        match open(node) {
            Ok(handle) => Box::into_raw(Box::new(ResilumWifiGroup(handle))),
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    })
}
