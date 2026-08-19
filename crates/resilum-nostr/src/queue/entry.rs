use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Inbound,
    Outbound,
}

/// A caller that cannot tell the two refusals apart logs a flood as a
/// duplicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "an entry the queue did not take is not owed to anyone"]
pub enum Queued {
    Held,
    AlreadyHeld,
    AtCeiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handoff {
    Direct { tries: u32 },
    LeftWithPropagationNode,
}

impl Default for Handoff {
    fn default() -> Self {
        Self::Direct { tries: 0 }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub direction: Direction,
    pub subscriber: [u8; 32],
    /// Copied from the registry when queued: a retry lands where the event arrived, not wherever the subscriber has moved since.
    pub lxmf: [u8; 16],
    pub event_id: [u8; 32],
    /// Shared, not owned: `due` clones every held entry each tick, and a body can run tens of KiB.
    pub event_json: Arc<str>,
    pub queued_at: i64,
    pub handoff: Handoff,
}
