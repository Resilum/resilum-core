use crate::coordinates::PeerId;
use crate::discovery::Attachments;
use crate::discovery::quota;

pub(super) fn who_announced(pubkey: &[u8]) -> Option<PeerId> {
    leviculum_std::api::Identity::from_public_key_bytes(pubkey)
        .ok()
        .map(|identity| *identity.hash())
}

pub(super) fn makes_room_for(
    attachments: &Attachments,
    mesh: usize,
    name: &str,
    peer: Option<PeerId>,
) -> bool {
    let newcomer = quota::Peer {
        attached_as: name.to_owned(),
        estimate: peer.and_then(|peer| attachments.estimate_of(&peer)),
    };
    match quota::judge(&attachments.kept(), newcomer, mesh) {
        quota::Verdict::Attach => true,
        quota::Verdict::Replace(displaced) => {
            tracing::info!(%displaced, %name, "a nearer peer took an attached one's place");
            attachments.release(&displaced);
            true
        }
        quota::Verdict::Refuse => {
            tracing::debug!(%name, "already attached to nearer and further peers");
            false
        }
    }
}
