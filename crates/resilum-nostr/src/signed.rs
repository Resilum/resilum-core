//! Signing a real event at any date, kind or address.

use data_encoding::HEXLOWER;
use k256::schnorr::SigningKey;
use k256::schnorr::signature::hazmat::PrehashSigner;
use sha2::{Digest, Sha256};

use crate::event::{self, Event};

/// The key is fixed and ephemeral, as a gift wrap's is: it says nothing about
/// who wrote what the event carries.
pub(crate) struct Draft<'a> {
    pub(crate) kind: u32,
    pub(crate) addressed_to: &'a [[u8; 32]],
    pub(crate) created_at: i64,
    pub(crate) content: &'a str,
}

pub(crate) fn sign(draft: &Draft<'_>) -> Event {
    let tags = draft
        .addressed_to
        .iter()
        .map(|to| vec!["p".to_string(), HEXLOWER.encode(to)])
        .collect();
    sign_tags(draft, tags)
}

/// Signing over `p` tags written exactly as given, so a test about how a tag
/// is spelled is not answered by the signature instead.
pub(crate) fn sign_tags(draft: &Draft<'_>, tags: Vec<Vec<String>>) -> Event {
    let key = signing_key();
    let pubkey = HEXLOWER.encode(&key.verifying_key().to_bytes());

    let canonical =
        serde_json::json!([0, pubkey, draft.created_at, draft.kind, tags, draft.content])
            .to_string();
    let id: [u8; 32] = Sha256::digest(&canonical).into();
    let sig = key.sign_prehash(&id).expect("a signature over the id");

    let json = serde_json::json!({
        "id": HEXLOWER.encode(&id),
        "pubkey": pubkey,
        "created_at": draft.created_at,
        "kind": draft.kind,
        "tags": tags,
        "content": draft.content,
        "sig": HEXLOWER.encode(&sig.to_bytes()),
    });
    event::parse(&json.to_string()).expect("parses")
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[3u8; 32].into()).expect("a valid signing key")
}

/// The key these fixtures are signed with, for a test that has to name it
/// inside the event's own tags and so cannot read it off afterwards.
pub(crate) fn pubkey() -> [u8; 32] {
    signing_key().verifying_key().to_bytes().into()
}

pub(crate) fn gift_wrap(addressed_to: [u8; 32], created_at: i64, content: &str) -> Event {
    gift_wrap_to(&[addressed_to], created_at, content)
}

pub(crate) fn gift_wrap_to(addressed_to: &[[u8; 32]], created_at: i64, content: &str) -> Event {
    sign(&Draft {
        kind: 1059,
        addressed_to,
        created_at,
        content,
    })
}

#[cfg(test)]
mod tests {
    /// A helper that produced junk would make every test that uses it pass by
    /// refusing its input for the wrong reason.
    #[test]
    fn the_fixtures_this_module_makes_are_genuinely_signed() {
        assert!(
            super::gift_wrap([7u8; 32], 1_000_000, "one")
                .verify()
                .is_ok()
        );
        assert!(
            super::gift_wrap([9u8; 32], 4_102_444_800, "two")
                .verify()
                .is_ok()
        );
        let note = super::sign(&super::Draft {
            kind: 1,
            addressed_to: &[[7u8; 32]],
            created_at: 1_000_000,
            content: "three",
        });
        assert!(note.verify().is_ok());

        let both = super::gift_wrap_to(&[[7u8; 32], [8u8; 32]], 1_000_000, "four");
        assert!(both.verify().is_ok());
        assert_eq!(both.tags.len(), 2);
    }
}
