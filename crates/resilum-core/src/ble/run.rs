mod dial;
mod meet;
mod rounds;

use std::sync::Arc;
use std::time::Duration;

use leviculum_std::driver::ReticulumNode;
use tokio::sync::mpsc::Receiver;

use super::beacon::Beacon;
use super::election::{Field, SomeoneElsesGroup};
use super::handshake::Handshakes;
use super::links::{Links, PeerId};
use super::radio::{Radio, RadioEvent};
use super::spec;
use dial::HeldOff;

pub use rounds::{Deciding, keep_deciding};

const LOOK_AROUND_EVERY: Duration = Duration::from_secs(5);
const GIVE_UP_ON_A_HANDSHAKE_AFTER_MS: u64 = 5_000;
const A_SCAN_THAT_HEARD_NOTHING_IS_RESTARTED_AFTER_MS: u64 = 120_000;

pub struct Ours {
    pub engine: Arc<ReticulumNode>,
    pub radio: Arc<dyn Radio>,
    pub identity: PeerId,
    pub beacon: Beacon,
    pub attachments: Arc<crate::discovery::Attachments>,
    pub origins: Arc<crate::discovery::OriginRegistry>,
    pub field: Field,
    pub someone_elses_group: SomeoneElsesGroup,
}

pub async fn run(ours: Ours, mut events: Receiver<RadioEvent>, since: std::time::Instant) {
    let now_ms = move || u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX);
    let mut links = Links::new();
    let mut waiting = Handshakes::default();
    let mut held_off = HeldOff::new();
    let mut look_around = tokio::time::interval(LOOK_AROUND_EVERY);
    let mut scanning_since = 0u64;

    loop {
        tokio::select! {
            heard = events.recv() => match heard {
                None => return,
                Some(event) => {
                    if matches!(event, RadioEvent::Seen { .. }) {
                        scanning_since = now_ms();
                    }
                    on_event(&ours, &mut links, &mut waiting, &mut held_off, event, now_ms());
                }
            },
            _ = look_around.tick() => {
                if a_scan_this_quiet_may_have_died(now_ms(), scanning_since) {
                    scanning_since = now_ms();
                    let _ = ours.radio.scan(spec::SERVICE);
                }
                ours.someone_elses_group.forget_it_if_it_has_gone_quiet(now_ms());
                dial::those_who_waited(&ours, &links, &mut held_off, now_ms());
                for conn in waiting.gave_up_by(now_ms(), GIVE_UP_ON_A_HANDSHAKE_AFTER_MS) {
                    ours.radio.disconnect(conn);
                }
            }
        }
    }
}

fn a_scan_this_quiet_may_have_died(now_ms: u64, scanning_since: u64) -> bool {
    now_ms.saturating_sub(scanning_since) >= A_SCAN_THAT_HEARD_NOTHING_IS_RESTARTED_AFTER_MS
}

fn on_event(
    ours: &Ours,
    links: &mut Links,
    waiting: &mut Handshakes,
    held_off: &mut HeldOff,
    event: RadioEvent,
    now_ms: u64,
) {
    match event {
        RadioEvent::Seen {
            address,
            name,
            beacon,
        } => {
            dial::on_seen(
                ours,
                links,
                held_off,
                address,
                Beacon::heard(&beacon, name.as_deref()),
                now_ms,
            );
        }
        RadioEvent::Connected {
            conn,
            address,
            role,
            ..
        } => {
            held_off.remove(&address);
            let _ = waiting.began(ours.radio.as_ref(), conn, address, role, now_ms);
        }
        RadioEvent::Data {
            conn,
            characteristic,
            value,
        } => {
            if waiting.is_waiting(conn) {
                meet::on_handshake(ours, links, waiting, conn, characteristic, &value, now_ms);
            } else {
                links.hand_over(conn, value);
            }
        }
        RadioEvent::Disconnected { conn } => {
            waiting.forget(conn);
            if let Some(peer) = links.part(conn) {
                ours.field.gone(peer);
            }
        }
        RadioEvent::WritableChanged { .. } => {}
    }
}

#[cfg(test)]
mod tests;
