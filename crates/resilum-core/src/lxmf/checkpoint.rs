//! Durable router state. `LxmfRouter::persist` runs under the core lock, so
//! this storage hands each change to a store instead of writing it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use leviculum_lxmf::storage::{LxmfStorage, StorageError};
use resilum_store::Document;

type Slots = BTreeMap<Vec<u8>, Vec<u8>>;

pub(super) struct Checkpoint {
    slots: Document<Slots>,
}

impl Checkpoint {
    pub(super) fn open(path: PathBuf) -> Self {
        Self {
            slots: Document::open_or_start_empty(path, Slots::new(), decode, encode),
        }
    }
}

impl LxmfStorage for Checkpoint {
    fn load(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.slots.read(|slots| slots.get(key).cloned()))
    }

    fn store(&mut self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        self.slots
            .change(|slots| slots.insert(key.to_vec(), value.to_vec()));
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), StorageError> {
        self.slots.change(|slots| slots.remove(key));
        Ok(())
    }

    fn keys(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>, StorageError> {
        Ok(self.slots.read(|slots| {
            slots
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect()
        }))
    }
}

fn decode(bytes: &[u8]) -> Result<Slots, String> {
    rmp_serde::from_slice(bytes).map_err(|e| e.to_string())
}

fn encode(slots: &Slots) -> Vec<u8> {
    rmp_serde::to_vec(slots).unwrap_or_default()
}
