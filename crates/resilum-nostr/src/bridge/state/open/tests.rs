//! Where the stores come from when the configured place for them is not
//! there, and when there was never meant to be one.

use super::*;
use crate::queue::{Direction, Entry, Handoff, Queued};
use crate::subscription::Subscription;

const RETENTION: Duration = Duration::from_secs(600);

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("resilum-nostr-open-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn a_storage_path_no_directory_can_be_created_under_refuses_to_open() {
    let root = scratch("uncreatable");
    std::fs::write(root.join("nostr"), "a file where the directory belongs").expect("seeds it");

    assert!(stores(Some(&root), RETENTION).is_err());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_node_with_no_storage_path_gets_stores_that_work_in_memory() {
    let (registry, queue) = stores(None, RETENTION).expect("opens ephemeral");

    registry
        .accept(
            Subscription {
                pubkey: [7u8; 32],
                lxmf: [3u8; 16],
                created_at: 100,
            },
            100,
        )
        .expect("accepted");
    assert_eq!(
        queue.push(Entry {
            direction: Direction::Inbound,
            subscriber: [7u8; 32],
            lxmf: [3u8; 16],
            event_id: [1u8; 32],
            event_json: "{}".into(),
            queued_at: 100,
            handoff: Handoff::default(),
        }),
        Queued::Held
    );

    assert_eq!(registry.live(100).len(), 1);
    assert_eq!(queue.due(100).len(), 1);
}
