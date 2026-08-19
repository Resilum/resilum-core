//! Validating a subscription request.
//!
//! Cheap structural checks run before the signature check: a request that is
//! not even shaped like a subscription should not cost an elliptic curve
//! operation.

use crate::event::{Event, decode_tag_hex};

pub(crate) const KIND: u32 = 30078;

/// How far ahead of us a subscriber's clock may be before we refuse it.
///
/// A request dated in the future would otherwise outlive the retention window
/// it is supposed to sit inside, and would beat a genuinely newer one past
/// the replay guard.
pub(crate) const FUTURE_SKEW_SECS: i64 = 900;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Subscription {
    pub(crate) pubkey: [u8; 32],
    pub(crate) lxmf: [u8; 16],
    pub(crate) created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refused {
    ClockSkew,
    Malformed(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClockSkew => f.write_str("created_at is too far in the future"),
            Self::Malformed(what) => f.write_str(what),
        }
    }
}

fn malformed(what: impl Into<String>) -> Refused {
    Refused::Malformed(what.into())
}

pub(crate) fn from_event(event: &Event, now: i64) -> Result<Subscription, Refused> {
    if event.kind != KIND {
        return Err(malformed(format!(
            "kind {} is not a subscription",
            event.kind
        )));
    }
    if event
        .tags
        .iter()
        .any(|t| t.first().is_some_and(|n| n == "filter"))
    {
        return Err(malformed("explicit filters are not accepted yet"));
    }
    if event.created_at > now + FUTURE_SKEW_SECS {
        return Err(Refused::ClockSkew);
    }
    let lxmf = event.tag("lxmf").ok_or_else(|| malformed("no lxmf tag"))?;
    let lxmf: [u8; 16] = decode_tag_hex(lxmf).map_err(|e| malformed(format!("lxmf tag: {e}")))?;
    let pubkey = pubkey_of(event)?;
    event.verify().map_err(|e| malformed(e.to_string()))?;
    Ok(Subscription {
        pubkey,
        lxmf,
        created_at: event.created_at,
    })
}

/// The signing key, having checked the human-readable `npub` names it too.
fn pubkey_of(event: &Event) -> Result<[u8; 32], Refused> {
    let signing = event
        .pubkey_bytes()
        .ok_or_else(|| malformed("pubkey is not 32 bytes of hex"))?;
    let npub = event.tag("npub").ok_or_else(|| malformed("no npub tag"))?;
    let claimed =
        crate::npub::to_pubkey(npub).ok_or_else(|| malformed("npub is not a valid bech32 npub"))?;
    if claimed != signing {
        return Err(malformed("npub tag disagrees with the signing key"));
    }
    Ok(signing)
}

#[cfg(test)]
mod tests;
