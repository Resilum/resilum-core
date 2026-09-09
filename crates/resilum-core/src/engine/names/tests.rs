use super::{kept, only_those_that_resolve, resolves};
use crate::Config;

fn named(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| String::from(*n)).collect()
}

#[test]
fn a_name_that_answers_is_kept_and_one_that_does_not_is_left_out() {
    let asked = named(&["here.example:1", "nowhere.example:2", "also.here:3"]);

    let surviving = kept(&asked, |name| name != "nowhere.example:2");

    assert_eq!(surviving, named(&["here.example:1", "also.here:3"]));
}

#[test]
fn the_order_anchors_were_given_in_is_the_order_they_are_dialled() {
    let asked = named(&["third:3", "first:1", "second:2"]);

    assert_eq!(kept(&asked, |_| true), asked);
}

#[test]
fn a_host_with_no_resolver_at_all_leaves_nothing_behind() {
    assert!(kept(&named(&["a:1", "b:2"]), |_| false).is_empty());
}

#[test]
fn a_literal_address_resolves_without_a_resolver() {
    assert!(resolves("127.0.0.1:4242"));
    assert!(resolves("[::1]:4242"));
}

#[test]
fn a_node_keeps_every_anchor_it_can_reach_by_address() {
    let config = Config {
        bootstrap: named(&["127.0.0.1:4242"]),
        bootstrap_only: named(&["[::1]:4343"]),
        ..Config::minimal("names")
    };

    let trimmed = only_those_that_resolve(&config);

    assert_eq!(trimmed.bootstrap, config.bootstrap);
    assert_eq!(trimmed.bootstrap_only, config.bootstrap_only);
}
