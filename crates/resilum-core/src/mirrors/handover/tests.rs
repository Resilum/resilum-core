use std::path::PathBuf;

use super::{Entry, SharedWithRngit, whom_to_ask};

fn a_peer_serving(rngit: &str, repos: &[&str]) -> Entry {
    Entry {
        peer: String::from("peer"),
        rngit: String::from(rngit),
        repos: repos.iter().map(|r| (*r).to_owned()).collect(),
        last_seen_unix: 0,
    }
}

fn named(what: &str) -> Vec<String> {
    vec![String::from(what)]
}

fn a_state_dir(test: &str, served: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("resilum-handover-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("state directory");
    std::fs::write(root.join("served"), served).expect("served");
    root
}

#[test]
fn we_announce_only_what_rngit_says_it_serves() {
    let state = a_state_dir("announce", "resilum-core\n");
    let shared = SharedWithRngit::beside(&state.join("destination"));

    let announced =
        shared.of_these_we_serve(&[String::from("resilum-core"), String::from("resilum-mobile")]);

    assert_eq!(announced, named("resilum-core"));
}

#[test]
fn a_repo_we_lack_is_asked_for_from_whoever_has_it() {
    let asking = whom_to_ask(
        &named("resilum-core"),
        &[],
        &[a_peer_serving("abcdef", &["resilum-core"])],
    );

    assert_eq!(
        asking,
        named("resilum-core\trns://abcdef/mirrors/resilum-core")
    );
}

#[test]
fn a_repo_we_already_serve_is_not_asked_for_again() {
    let asking = whom_to_ask(
        &named("resilum-core"),
        &named("resilum-core"),
        &[a_peer_serving("abcdef", &["resilum-core"])],
    );

    assert!(asking.is_empty());
}

#[test]
fn a_repo_nobody_advertises_cannot_be_asked_for() {
    let asking = whom_to_ask(
        &named("resilum-core"),
        &[],
        &[a_peer_serving("abcdef", &["something-else"])],
    );

    assert!(asking.is_empty());
}
