use super::*;
use crate::config::EgressListen;

fn cand(hash: u8, service: &str, country: &str) -> Candidate {
    let mut c = Candidate::new(vec![hash], service);
    c.exit_country = country.into();
    c
}

fn ours(service: &str, hash: u8) -> OwnExits {
    OwnExits::of(&[EgressListen::new(service, None)], |_| vec![hash])
}

#[test]
fn an_exit_that_failed_its_last_probe_carries_no_traffic_but_stays_probeable() {
    let mut c = cand(1, "tor", "*");
    c.healthy = false;

    assert!(
        eligible(
            std::slice::from_ref(&c),
            "smart",
            &[],
            &[],
            &OwnExits::default()
        )
        .is_empty()
    );
    assert_eq!(
        allowed(&[c], "smart", &[], &[], &OwnExits::default()).len(),
        1,
        "otherwise nothing ever probes it again and it can never recover"
    );
}

#[test]
fn smart_excludes_own_socks_egress_only() {
    let socks = cand(1, "socks-egress", "*");
    let tor = cand(1, "tor", "*");
    assert!(eligible(&[socks], "smart", &[], &[], &ours("socks-egress", 1)).is_empty());
    assert_eq!(
        eligible(&[tor], "smart", &[], &[], &ours("tor", 1)).len(),
        1
    );
}

#[test]
fn use_own_true_and_false() {
    let c = cand(1, "socks-egress", "*");
    assert_eq!(
        eligible(
            std::slice::from_ref(&c),
            "true",
            &[],
            &[],
            &ours("socks-egress", 1)
        )
        .len(),
        1
    );
    assert!(eligible(&[c], "false", &[], &[], &ours("socks-egress", 1)).is_empty());
}

#[test]
fn country_allow_deny_and_unknown() {
    let de = || vec!["DE".to_owned()];
    let unknown = cand(1, "tor", "*");
    let de_cand = cand(2, "tor", "DE");
    assert!(
        eligible(
            std::slice::from_ref(&de_cand),
            "smart",
            &[],
            &de(),
            &OwnExits::default()
        )
        .is_empty()
    );
    assert_eq!(
        eligible(&[de_cand], "smart", &de(), &[], &OwnExits::default()).len(),
        1
    );
    assert!(eligible(&[unknown], "smart", &de(), &[], &OwnExits::default()).is_empty());
}
