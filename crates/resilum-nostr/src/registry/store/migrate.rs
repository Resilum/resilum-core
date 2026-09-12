//! Giving a batch to a record that predates batching.
//!
//! Silently treating a missing batch as `0` would pile every subscriber a
//! prior version wrote into one batch and overflow it. Instead each such
//! record is placed by the same rule a fresh subscription follows at accept
//! time, applied once, in file order, at load. It does not weigh liveness:
//! this runs before `Registry` has a "now" to weigh it against, and a batch
//! chosen a little conservatively here costs nothing but headroom.

use crate::registry::admit::lowest_open;

use super::line::Loaded;
use super::{Held, Record};

pub(super) struct Migration {
    pub(super) held: Held,
    pub(super) rewrite_needed: bool,
}

pub(super) fn assign(loaded: Vec<Loaded>) -> Migration {
    let mut held = Held::default();
    let mut rewrite_needed = false;
    for entry in loaded {
        let batch = entry.batch.unwrap_or_else(|| {
            rewrite_needed = true;
            lowest_open(&held, |_| true)
        });
        held.insert(
            entry.pubkey,
            Record {
                lxmf: entry.lxmf,
                created_at: entry.created_at,
                last_seen: entry.last_seen,
                batch,
            },
        );
    }
    Migration {
        held,
        rewrite_needed,
    }
}

#[cfg(test)]
#[path = "migrate_tests.rs"]
mod tests;
