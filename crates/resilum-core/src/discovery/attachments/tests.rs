use std::sync::Arc;

use super::{Attached, Attachments};
use crate::coordinates::{Coordinates, PeerId};

const WARMED_FROM_CACHE: &str = "YggdrasilDiscovered[200:d5a7::1]:4242";

fn attachments() -> Attachments {
    Attachments::new(Arc::new(Coordinates::default()))
}

fn anonymous() -> Attached {
    Attached {
        service: "yggdrasil".to_owned(),
        announced_by: None,
        interface: leviculum_std::InterfaceId(7),
        _detaches_when_dropped: Box::new(()),
    }
}

fn a_peer() -> PeerId {
    [0xab; 16]
}

#[test]
fn a_peer_attached_before_anyone_named_it_is_no_link() {
    let held = attachments();
    held.hold(WARMED_FROM_CACHE.to_owned(), anonymous());

    assert!(held.links().is_empty());
}

#[test]
fn an_announce_names_a_peer_that_was_attached_from_cache() {
    let held = attachments();
    held.hold(WARMED_FROM_CACHE.to_owned(), anonymous());

    assert!(held.learn_who_announced(WARMED_FROM_CACHE, a_peer()));

    let links = held.links();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].peer, a_peer());
    assert_eq!(links[0].transport, "yggdrasil");
}

#[test]
fn a_second_announce_does_not_rename_a_peer_we_already_know() {
    let held = attachments();
    held.hold(WARMED_FROM_CACHE.to_owned(), anonymous());
    held.learn_who_announced(WARMED_FROM_CACHE, a_peer());

    assert!(!held.learn_who_announced(WARMED_FROM_CACHE, [0xcd; 16]));
    assert_eq!(held.links()[0].peer, a_peer());
}

#[test]
fn a_peer_redialling_the_same_way_keeps_the_name_it_already_had() {
    let held = attachments();
    held.hold(WARMED_FROM_CACHE.to_owned(), anonymous());
    held.learn_who_announced(WARMED_FROM_CACHE, a_peer());

    held.hold(WARMED_FROM_CACHE.to_owned(), anonymous());

    assert_eq!(held.links()[0].peer, a_peer());
}

#[test]
fn naming_something_we_never_attached_changes_nothing() {
    let held = attachments();

    assert!(!held.learn_who_announced("nobody", a_peer()));
    assert!(held.links().is_empty());
}
