//! NIP-01 events: the only thing that crosses both networks unchanged.

use data_encoding::{HEXLOWER, HEXLOWER_PERMISSIVE};
use k256::schnorr::signature::hazmat::PrehashVerifier;
use k256::schnorr::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const GIFT_WRAP_KIND: u32 = 1059;
pub(crate) const LEGACY_DM_KIND: u32 = 4;
pub(crate) const DM_INBOX_RELAYS_KIND: u32 = 10050;

pub(crate) const NIP59_BACKDATE: i64 = 2 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Event {
    pub(crate) id: String,
    pub(crate) pubkey: String,
    pub(crate) created_at: i64,
    pub(crate) kind: u32,
    pub(crate) tags: Vec<Vec<String>>,
    pub(crate) content: String,
    pub(crate) sig: String,
}

#[derive(Debug)]
pub(crate) struct MalformedEvent(serde_json::Error);

/// A caller deciding whether to drop an event quietly, warn, or answer the
/// sender has to tell these faults apart.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum VerifyError {
    IdNotHex,
    IdMismatch,
    PubkeyNotHex,
    PubkeyOffCurve,
    SignatureNotHex,
    SignatureMalformed,
    SignatureMismatch,
}

impl VerifyError {
    fn reason(&self) -> &'static str {
        match self {
            Self::IdNotHex => "id is not 32 bytes of lowercase hex",
            Self::IdMismatch => "id does not match the event",
            Self::PubkeyNotHex => "pubkey is not 32 bytes of lowercase hex",
            Self::PubkeyOffCurve => "pubkey is not on the curve",
            Self::SignatureNotHex => "signature is not hex",
            Self::SignatureMalformed => "signature is malformed",
            Self::SignatureMismatch => "signature does not match",
        }
    }
}

impl std::fmt::Display for MalformedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a nostr event: {}", self.0)
    }
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason())
    }
}

impl std::error::Error for MalformedEvent {}
impl std::error::Error for VerifyError {}

pub(crate) fn parse(json: &str) -> Result<Event, MalformedEvent> {
    serde_json::from_str(json).map_err(MalformedEvent)
}

impl Event {
    /// The id is a commitment to every field but the signature, so checking
    /// it first means the signature is checked over what we actually read.
    pub(crate) fn verify(&self) -> Result<(), VerifyError> {
        let computed = Sha256::digest(self.canonical());
        let claimed = self.id_bytes().ok_or(VerifyError::IdNotHex)?;
        if computed.as_slice() != claimed {
            return Err(VerifyError::IdMismatch);
        }
        let pubkey = self.pubkey_bytes().ok_or(VerifyError::PubkeyNotHex)?;
        let key = VerifyingKey::try_from(&pubkey[..]).map_err(|_| VerifyError::PubkeyOffCurve)?;
        let raw = HEXLOWER
            .decode(self.sig.as_bytes())
            .map_err(|_| VerifyError::SignatureNotHex)?;
        let sig = Signature::try_from(&raw[..]).map_err(|_| VerifyError::SignatureMalformed)?;
        key.verify_prehash(&claimed, &sig)
            .map_err(|_| VerifyError::SignatureMismatch)
    }

    pub(crate) fn id_bytes(&self) -> Option<[u8; 32]> {
        decode_hex(&self.id).ok()
    }

    pub(crate) fn pubkey_bytes(&self) -> Option<[u8; 32]> {
        decode_hex(&self.pubkey).ok()
    }

    pub(crate) fn tag(&self, name: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|t| t.first().is_some_and(|n| n == name))
            .and_then(|t| t.get(1))
            .map(String::as_str)
    }

    /// NIP-01 fixes this array and its compact encoding; `serde_json` emits
    /// exactly it — no spaces, and only the escapes the spec names.
    fn canonical(&self) -> String {
        serde_json::json!([
            0,
            self.pubkey,
            self.created_at,
            self.kind,
            self.tags,
            self.content
        ])
        .to_string()
    }
}

/// Every fixed-width field NIP-01 defines, and every identifier the stores
/// write down again, is lowercase hex of one exact length.
pub(crate) fn decode_hex<const N: usize>(hex: &str) -> Result<[u8; N], String> {
    HEXLOWER
        .decode(hex.as_bytes())
        .map_err(|e| e.to_string())?
        .try_into()
        .map_err(|_| format!("expected {N} bytes"))
}

/// The same, for a value someone else wrote into a tag. Casing carries no
/// meaning there — an uppercase key names the same person — and refusing it
/// costs that person the message rather than costing its author anything.
pub(crate) fn decode_tag_hex<const N: usize>(hex: &str) -> Result<[u8; N], String> {
    HEXLOWER_PERMISSIVE
        .decode(hex.as_bytes())
        .map_err(|e| e.to_string())?
        .try_into()
        .map_err(|_| format!("expected {N} bytes"))
}

#[cfg(test)]
mod tests;
