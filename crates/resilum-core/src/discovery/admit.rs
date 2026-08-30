use crate::coordinates::PeerId;
use crate::discovery::attachments::{Attached, Attachments};
use crate::discovery::quota;

pub(crate) enum Room {
    Yes,
    OnceThisIsLetGo(Attached),
    No,
}

pub(crate) fn who_announced(pubkey: &[u8]) -> Option<PeerId> {
    leviculum_std::api::Identity::from_public_key_bytes(pubkey)
        .ok()
        .map(|identity| *identity.hash())
}

pub(crate) fn room_for(
    attachments: &Attachments,
    mesh: usize,
    name: &str,
    peer: Option<PeerId>,
) -> Room {
    let newcomer = quota::Peer {
        attached_as: name.to_owned(),
        estimate: peer.and_then(|peer| attachments.estimate_of(&peer)),
        node: peer,
    };
    match quota::judge(&attachments.kept(), newcomer, mesh) {
        quota::Verdict::Attach => Room::Yes,
        quota::Verdict::Replace(displaced) => {
            tracing::info!(%displaced, %name, "a nearer peer took an attached one's place");
            match attachments.release(&displaced) {
                Some(gone) => Room::OnceThisIsLetGo(gone),
                None => Room::Yes,
            }
        }
        quota::Verdict::Refuse => {
            tracing::debug!(%name, "already attached to nearer and further peers");
            Room::No
        }
    }
}
