use serde_json::json;

use crate::registry::AcceptError;
use crate::subscription::Refused;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::bridge) enum Refusal {
    Malformed,
    ClockSkew,
    NotCarried,
    Replayed,
    Stale,
    Full,
}

/// What the device should do about it, kept apart from the cause so a cause
/// added later needs no change on the receiving side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Retry {
    Fix,
    Later,
    Elsewhere,
    Never,
}

impl Refusal {
    fn reason(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::ClockSkew => "clock_skew",
            Self::NotCarried => "not_carried",
            Self::Replayed => "replayed",
            Self::Stale => "stale",
            Self::Full => "full",
        }
    }

    fn retry(self) -> Retry {
        match self {
            Self::Malformed => Retry::Never,
            Self::ClockSkew | Self::Stale => Retry::Fix,
            Self::Replayed => Retry::Later,
            Self::NotCarried | Self::Full => Retry::Elsewhere,
        }
    }
}

pub(in crate::bridge) enum Outcome {
    Accepted,
    Refused(Refusal),
}

impl Outcome {
    pub(in crate::bridge) fn json(self) -> String {
        match self {
            Self::Accepted => json!({ "result": "accepted" }).to_string(),
            Self::Refused(refusal) => json!({
                "result": "refused",
                "reason": refusal.reason(),
                "retry": refusal.retry().token(),
            })
            .to_string(),
        }
    }
}

impl Retry {
    fn token(self) -> &'static str {
        match self {
            Self::Fix => "fix",
            Self::Later => "later",
            Self::Elsewhere => "elsewhere",
            Self::Never => "never",
        }
    }
}

impl From<&Refused> for Refusal {
    fn from(refused: &Refused) -> Self {
        match refused {
            Refused::ClockSkew => Self::ClockSkew,
            Refused::Malformed(_) => Self::Malformed,
        }
    }
}

impl From<&AcceptError> for Refusal {
    fn from(error: &AcceptError) -> Self {
        match error {
            AcceptError::Lapsed => Self::Stale,
            AcceptError::Replayed => Self::Replayed,
            AcceptError::Full { .. } => Self::Full,
        }
    }
}

#[cfg(test)]
mod tests;
