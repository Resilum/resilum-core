use std::net::{Ipv6Addr, SocketAddrV6, UdpSocket};

use super::{WHAT_RETICULUM_EXPECTS, nobody_else_holds};

fn a_port_we_hold() -> (UdpSocket, u16) {
    let held = UdpSocket::bind(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0)).expect("bind");
    let port = held.local_addr().expect("addr").port();
    (held, port)
}

#[test]
fn a_free_port_is_the_one_we_asked_for() {
    let (giving_it_up, port) = a_port_we_hold();
    drop(giving_it_up);

    assert_eq!(nobody_else_holds(port), port);
}

#[test]
fn a_port_someone_else_holds_is_given_up_for_another() {
    let (_still_held, port) = a_port_we_hold();

    let ours = nobody_else_holds(port);

    assert_ne!(ours, port);
    assert_ne!(ours, 0);
}

#[test]
fn the_one_reticulum_expects_is_what_we_ask_for_first() {
    assert_eq!(WHAT_RETICULUM_EXPECTS, 42671);
}
