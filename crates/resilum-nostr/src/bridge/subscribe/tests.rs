use std::time::Duration;

use super::*;

const STRANGER: [u8; 32] = [0xab; 32];
const SOMEONE_ELSES_NPUB: &str = "npub1zutzeysacnf9rru6zqwmxd54mud0k44tst6l70ja5mhv8jjumytsd2x7nu";

fn a_personal_bridge() -> (Registry, NostrConfig) {
    (
        Registry::ephemeral(Duration::from_secs(60)),
        NostrConfig {
            allow_npubs: vec![SOMEONE_ELSES_NPUB.to_owned()],
            ..NostrConfig::default()
        },
    )
}

fn request_from(pubkey: [u8; 32], created_at: i64) -> Subscription {
    Subscription {
        pubkey,
        lxmf: [0x11; 16],
        created_at,
    }
}

#[test]
fn a_stale_request_is_refused_as_stale_rather_than_as_one_this_bridge_does_not_carry() {
    let (registry, cfg) = a_personal_bridge();

    let refusal = refusal_for(&registry, &cfg, &request_from(STRANGER, 0), 10_000);

    assert_eq!(refusal.map(|(refusal, _)| refusal), Some(Refusal::Stale));
}

#[test]
fn a_fresh_request_from_an_unlisted_npub_is_still_refused_as_not_carried() {
    let (registry, cfg) = a_personal_bridge();

    let refusal = refusal_for(&registry, &cfg, &request_from(STRANGER, 10_000), 10_000);

    assert_eq!(
        refusal.map(|(refusal, _)| refusal),
        Some(Refusal::NotCarried)
    );
}

#[test]
fn an_open_bridge_refuses_nothing_a_fresh_request_names() {
    let registry = Registry::ephemeral(Duration::from_secs(60));

    let refusal = refusal_for(
        &registry,
        &NostrConfig::default(),
        &request_from(STRANGER, 10_000),
        10_000,
    );

    assert_eq!(refusal, None);
}
