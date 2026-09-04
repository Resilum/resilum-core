use std::net::{Ipv4Addr, TcpListener};

use resilum_core::WifiGroup;

use super::knocked_until_the_owner_answers;

fn a_listener_standing_in_for_the_owner() -> (TcpListener, WifiGroup) {
    let listening = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port to stand in on");
    let port = listening.local_addr().expect("its address").port();
    (
        listening,
        WifiGroup {
            owner_address: Ipv4Addr::LOCALHOST,
            port,
            ..WifiGroup::default()
        },
    )
}

#[test]
fn we_knock_where_the_owner_said_it_would_be() {
    let (_listening, group) = a_listener_standing_in_for_the_owner();

    let open = knocked_until_the_owner_answers(&group).expect("the owner answered");

    assert_eq!(
        open.peer_addr().expect("who answered").port(),
        group.port,
        "we reached something other than the owner"
    );
}
