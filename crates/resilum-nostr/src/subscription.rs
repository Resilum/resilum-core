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

pub(crate) fn from_event(event: &Event, now: i64) -> Result<Subscription, String> {
    if event.kind != KIND {
        return Err(format!("kind {} is not a subscription", event.kind));
    }
    if event
        .tags
        .iter()
        .any(|t| t.first().is_some_and(|n| n == "filter"))
    {
        return Err("explicit filters are not accepted yet".into());
    }
    if event.created_at > now + FUTURE_SKEW_SECS {
        return Err("created_at is too far in the future".into());
    }
    let lxmf = event.tag("lxmf").ok_or("no lxmf tag")?;
    let lxmf: [u8; 16] = decode_tag_hex(lxmf).map_err(|e| format!("lxmf tag: {e}"))?;
    let pubkey = pubkey_of(event)?;
    event.verify().map_err(|e| e.to_string())?;
    Ok(Subscription {
        pubkey,
        lxmf,
        created_at: event.created_at,
    })
}

/// The signing key, having checked the human-readable `npub` names it too.
fn pubkey_of(event: &Event) -> Result<[u8; 32], String> {
    let signing = event
        .pubkey_bytes()
        .ok_or("pubkey is not 32 bytes of hex")?;
    let npub = event.tag("npub").ok_or("no npub tag")?;
    let claimed = crate::npub::to_pubkey(npub).ok_or("npub is not a valid bech32 npub")?;
    if claimed != signing {
        return Err("npub tag disagrees with the signing key".into());
    }
    Ok(signing)
}

#[cfg(test)]
mod tests;
