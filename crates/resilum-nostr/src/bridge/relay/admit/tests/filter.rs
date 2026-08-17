//! Each test here pairs a refusal with a control that differs in one thing
//! only, so a check that stopped mattering fails the pair rather than
//! passing on a fixture something earlier had already thrown out.

use super::super::admit;
use super::{Held, NOW, SUBSCRIBER};
use crate::config::NostrConfig;
use crate::signed::{Draft, gift_wrap, sign, sign_tags};

#[test]
fn an_event_past_the_ceiling_is_refused_before_it_costs_a_slot() {
    let held = Held::new();
    let huge = gift_wrap(SUBSCRIBER, NOW, &"x".repeat(128 * 1024));

    assert!(huge.verify().is_ok(), "the size is the only thing wrong");
    assert!(admit(&held.stores(), SUBSCRIBER, &huge, NOW).is_none());

    let small = gift_wrap(SUBSCRIBER, NOW, "x");
    assert!(admit(&held.stores(), SUBSCRIBER, &small, NOW).is_some());
}

/// The `REQ` asked for the kinds this bridge carries. A relay that answers
/// with something else is answering a question nobody put to it.
#[test]
fn an_event_of_a_kind_this_bridge_does_not_carry_is_refused() {
    let held = Held::new();
    let note = sign(&Draft {
        kind: 1,
        addressed_to: &[SUBSCRIBER],
        created_at: NOW,
        content: "a public note",
    });

    assert!(note.verify().is_ok(), "the kind is the only thing wrong");
    assert!(admit(&held.stores(), SUBSCRIBER, &note, NOW).is_none());

    let wrapped = gift_wrap(SUBSCRIBER, NOW, "a public note");
    assert!(admit(&held.stores(), SUBSCRIBER, &wrapped, NOW).is_some());
}

/// `legacy_dm: false` stops the bridge asking for kind 4; on its own that
/// does not stop a relay sending one anyway.
#[test]
fn a_bridge_with_legacy_delivery_off_refuses_a_kind_four_message() {
    let legacy = sign(&Draft {
        kind: 4,
        addressed_to: &[SUBSCRIBER],
        created_at: NOW,
        content: "an older client",
    });

    let strict = Held::configured(NostrConfig {
        legacy_dm: false,
        ..NostrConfig::default()
    });
    assert!(admit(&strict.stores(), SUBSCRIBER, &legacy, NOW).is_none());

    let widened = Held::new();
    assert!(admit(&widened.stores(), SUBSCRIBER, &legacy, NOW).is_some());
}

#[test]
fn an_event_addressed_to_someone_else_is_not_owed_to_this_subscriber() {
    let held = Held::new();
    let elsewhere = gift_wrap([9u8; 32], NOW, "for another mailbox");

    assert!(
        elsewhere.verify().is_ok(),
        "the p tag is the only thing wrong"
    );
    assert!(admit(&held.stores(), SUBSCRIBER, &elsewhere, NOW).is_none());

    let ours = gift_wrap(SUBSCRIBER, NOW, "for another mailbox");
    assert!(admit(&held.stores(), SUBSCRIBER, &ours, NOW).is_some());
}

/// This event alone would not reach us: `frames` writes `#p` filter values in
/// lowercase and NIP-01 matches tag filters as exact strings. What the
/// permissive read rescues is the batched case — one event carrying a
/// lowercase `p` for the subscriber it arrived for and an uppercase one for
/// another in the same batch, who is owed it too.
#[test]
fn a_p_tag_in_uppercase_still_names_its_subscriber() {
    // Letters, so that upper and lower case are actually different strings.
    let lettered = [0xabu8; 32];
    let held = Held::holding(&[lettered]);
    let shouted = tagged(&data_encoding::HEXLOWER.encode(&lettered).to_uppercase());

    assert!(shouted.verify().is_ok(), "the casing is the only oddity");
    assert!(admit(&held.stores(), lettered, &shouted, NOW).is_some());

    let spoken = tagged(&data_encoding::HEXLOWER.encode(&lettered));
    assert!(admit(&held.stores(), lettered, &spoken, NOW).is_some());
}

fn tagged(p: &str) -> crate::event::Event {
    sign_tags(
        &Draft {
            kind: 1059,
            addressed_to: &[],
            created_at: NOW,
            content: "spelled loudly",
        },
        vec![vec!["p".to_string(), p.to_string()]],
    )
}
