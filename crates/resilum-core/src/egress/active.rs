//! Live connect-side links per egress destination, so links to a peer that
//! stops being compatible can be torn down.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use leviculum_std::api::LinkId;
use leviculum_std::driver::ReticulumNode;

#[derive(Default)]
pub struct ActiveLinks {
    by_dest: Mutex<HashMap<[u8; 16], HashSet<LinkId>>>,
}

impl ActiveLinks {
    pub fn register(&self, dest_hash: [u8; 16], link_id: LinkId) {
        self.by_dest
            .lock()
            .unwrap()
            .entry(dest_hash)
            .or_default()
            .insert(link_id);
    }

    pub fn deregister(&self, dest_hash: &[u8; 16], link_id: &LinkId) {
        let mut map = self.by_dest.lock().unwrap();
        if let Some(set) = map.get_mut(dest_hash) {
            set.remove(link_id);
            if set.is_empty() {
                map.remove(dest_hash);
            }
        }
    }

    pub async fn teardown_for(&self, engine: &Arc<ReticulumNode>, dest_hash: &[u8; 16]) {
        let ids: Vec<LinkId> = self
            .by_dest
            .lock()
            .unwrap()
            .remove(dest_hash)
            .map(|set| set.into_iter().collect())
            .unwrap_or_default();
        for id in ids {
            let mut handle = engine.link_handle(&id);
            let _ = handle.close().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_then_deregister_empties_the_entry() {
        let active = ActiveLinks::default();
        let id = LinkId::new([1; 16]);
        active.register([9; 16], id);
        assert!(active.by_dest.lock().unwrap().contains_key(&[9; 16]));

        active.deregister(&[9; 16], &id);
        assert!(active.by_dest.lock().unwrap().is_empty());
    }
}
