use std::sync::Arc;

use leviculum_std::api::{Destination, DestinationHash};
use leviculum_std::driver::ReticulumNode;

use crate::coordinates::PeerId;
use crate::discovery::Attachments;

pub(super) fn to_ask(
    engine: &Arc<ReticulumNode>,
    aspect: &[u8; 10],
    attachments: &Attachments,
    round: usize,
) -> Vec<(PeerId, DestinationHash)> {
    let kept = attachments.whose_links_we_keep();
    let (mut whose_links_we_keep, rest): (Vec<_>, Vec<_>) = reachable_peers(engine, aspect)
        .into_iter()
        .partition(|(peer, _)| kept.contains(peer));
    if whose_links_we_keep.is_empty() {
        return rest;
    }
    whose_links_we_keep.extend(one_of_the_rest_in_turn(&rest, round));
    whose_links_we_keep
}

fn one_of_the_rest_in_turn(
    rest: &[(PeerId, DestinationHash)],
    round: usize,
) -> Option<(PeerId, DestinationHash)> {
    rest.get(round % rest.len().max(1)).copied()
}

fn reachable_peers(
    engine: &Arc<ReticulumNode>,
    aspect: &[u8; 10],
) -> Vec<(PeerId, DestinationHash)> {
    let mut asking = Vec::new();
    for entry in engine.path_table_entries() {
        let Some(identity) = engine.get_identity(&entry.hash.into()) else {
            continue;
        };
        let at = Destination::compute_destination_hash(aspect, identity.hash());
        if engine.has_path(&at) {
            asking.push((*identity.hash(), at));
        }
    }
    asking.sort_unstable();
    asking.dedup();
    asking
}
