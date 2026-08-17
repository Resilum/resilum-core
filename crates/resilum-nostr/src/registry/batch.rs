//! Which `REQ` a subscriber's filter rides in.

/// A newtype because this number travels alongside plain `usize` counts —
/// filters per request, subscribers held — that it must never be swapped
/// with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BatchId(usize);

impl BatchId {
    pub(crate) const FIRST: Self = Self(0);

    pub(crate) fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub(crate) fn from_stored(raw: usize) -> Self {
        Self(raw)
    }

    pub(crate) fn stored(self) -> usize {
        self.0
    }
}

impl std::fmt::Display for BatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Where a subscription belongs, and whether its `REQ` has to be reissued.
pub(super) enum Placement {
    Refreshed(BatchId),
    Joined(BatchId),
}

impl Placement {
    pub(super) fn batch(&self) -> BatchId {
        match self {
            Self::Refreshed(batch) | Self::Joined(batch) => *batch,
        }
    }

    pub(super) fn joined(&self) -> Option<BatchId> {
        match self {
            Self::Joined(batch) => Some(*batch),
            Self::Refreshed(_) => None,
        }
    }
}
