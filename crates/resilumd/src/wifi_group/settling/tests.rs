use std::cell::Cell;
use std::net::Ipv4Addr;

use super::took_hold;

const OWNER: Ipv4Addr = Ipv4Addr::new(192, 168, 49, 1);

#[test]
fn an_address_already_there_is_not_waited_for() {
    let looks = Cell::new(0);

    let settled = took_hold(OWNER, || {
        looks.set(looks.get() + 1);
        Some(OWNER)
    });

    assert!(settled.is_ok());
    assert_eq!(looks.get(), 1);
}

#[test]
fn an_interface_still_coming_up_is_waited_for() {
    let looks = Cell::new(0);

    let settled = took_hold(OWNER, || {
        looks.set(looks.get() + 1);
        (looks.get() >= 3).then_some(OWNER)
    });

    assert!(settled.is_ok());
    assert_eq!(looks.get(), 3);
}

#[test]
fn an_interface_that_never_takes_it_reports_what_it_holds_instead() {
    let someone_elses = Ipv4Addr::new(192, 168, 88, 22);

    let settled = took_hold(OWNER, || Some(someone_elses));

    assert_eq!(settled, Err(Some(someone_elses)));
}

#[test]
fn an_interface_that_is_not_there_reports_no_address() {
    assert_eq!(took_hold(OWNER, || None), Err(None));
}
