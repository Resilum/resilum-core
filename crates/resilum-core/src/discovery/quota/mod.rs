use std::collections::BTreeSet;
use std::time::Duration;

const FEWEST_KEPT: usize = 4;
const MOST_KEPT: usize = 24;

/// `log2` of the mesh, which is what greedy routing over a small-world graph
/// needs per node to keep the diameter logarithmic (Kleinberg).
pub(crate) fn kept_of(mesh_destinations: usize) -> usize {
    let ideal = usize::BITS - mesh_destinations.max(1).leading_zeros();
    (ideal as usize).clamp(FEWEST_KEPT, MOST_KEPT)
}

fn crossing_of(kept: usize) -> usize {
    (kept / 4).max(1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Peer {
    pub(crate) attached_as: String,
    pub(crate) estimate: Option<Duration>,
    pub(crate) node: Option<[u8; 16]>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Attach,
    Replace(String),
    Refuse,
}

pub(crate) fn judge(kept: &[Peer], newcomer: Peer, mesh: usize) -> Verdict {
    let room = kept_of(mesh);
    if another_way_to_a_node_we_keep(kept, &newcomer) {
        return Verdict::Attach;
    }
    if nodes_kept(kept) < room {
        return Verdict::Attach;
    }
    let mut all = kept.to_vec();
    all.push(newcomer.clone());
    let worth_keeping = worth_keeping(&all, room);
    if !worth_keeping.contains(&newcomer.attached_as) {
        return Verdict::Refuse;
    }
    match kept
        .iter()
        .find(|peer| !worth_keeping.contains(&peer.attached_as))
    {
        Some(displaced) => Verdict::Replace(displaced.attached_as.clone()),
        None => Verdict::Refuse,
    }
}

fn another_way_to_a_node_we_keep(kept: &[Peer], newcomer: &Peer) -> bool {
    newcomer
        .node
        .is_some_and(|node| kept.iter().any(|peer| peer.node == Some(node)))
}

fn nodes_kept(kept: &[Peer]) -> usize {
    let named: BTreeSet<[u8; 16]> = kept.iter().filter_map(|peer| peer.node).collect();
    named.len() + kept.iter().filter(|peer| peer.node.is_none()).count()
}

fn worth_keeping(all: &[Peer], room: usize) -> BTreeSet<String> {
    let crossing = crossing_of(room);
    let mut placed: Vec<&Peer> = all.iter().filter(|peer| peer.estimate.is_some()).collect();
    placed.sort_by_key(|peer| peer.estimate);
    let nearest = placed.iter().take(room.saturating_sub(crossing));
    let furthest = placed.iter().rev().take(crossing);
    let mut keeping: BTreeSet<String> = nearest
        .chain(furthest)
        .map(|peer| peer.attached_as.clone())
        .collect();
    for unplaced in all.iter().filter(|peer| peer.estimate.is_none()) {
        if keeping.len() >= room {
            break;
        }
        keeping.insert(unplaced.attached_as.clone());
    }
    keeping
}

#[cfg(test)]
mod tests;
