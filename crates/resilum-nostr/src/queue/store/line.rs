use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};

use crate::event::decode_hex;
use crate::queue::{Direction, Entry, Handoff};

#[derive(Deserialize)]
struct Line {
    direction: String,
    subscriber: String,
    lxmf: String,
    event_id: String,
    event_json: String,
    queued_at: i64,
    #[serde(default)]
    direct_tries: u32,
    #[serde(default)]
    left_with_propagation_node: bool,
}

/// Borrowed so a flush does not copy every held event body, which is what the
/// entries' shared `Arc<str>` exists to avoid.
#[derive(Serialize)]
struct LineRef<'a> {
    direction: &'a str,
    subscriber: String,
    lxmf: String,
    event_id: String,
    event_json: &'a str,
    queued_at: i64,
    direct_tries: u32,
    left_with_propagation_node: bool,
}

pub(super) fn decode(line: &str) -> Result<Entry, String> {
    let parsed: Line = serde_json::from_str(line).map_err(|e| e.to_string())?;
    Ok(Entry {
        direction: decode_direction(&parsed.direction)?,
        subscriber: decode_hex::<32>(&parsed.subscriber)?,
        lxmf: decode_hex::<16>(&parsed.lxmf)?,
        event_id: decode_hex::<32>(&parsed.event_id)?,
        event_json: parsed.event_json.into(),
        queued_at: parsed.queued_at,
        handoff: if parsed.left_with_propagation_node {
            Handoff::LeftWithPropagationNode
        } else {
            Handoff::Direct {
                tries: parsed.direct_tries,
            }
        },
    })
}

pub(super) fn encode(entry: &Entry) -> Result<String, serde_json::Error> {
    serde_json::to_string(&LineRef {
        direction: encode_direction(entry.direction),
        subscriber: HEXLOWER.encode(&entry.subscriber),
        lxmf: HEXLOWER.encode(&entry.lxmf),
        event_id: HEXLOWER.encode(&entry.event_id),
        event_json: &entry.event_json,
        queued_at: entry.queued_at,
        direct_tries: match entry.handoff {
            Handoff::Direct { tries } => tries,
            Handoff::LeftWithPropagationNode => 0,
        },
        left_with_propagation_node: entry.handoff == Handoff::LeftWithPropagationNode,
    })
}

fn decode_direction(direction: &str) -> Result<Direction, String> {
    match direction {
        "inbound" => Ok(Direction::Inbound),
        "outbound" => Ok(Direction::Outbound),
        other => Err(format!("unknown direction {other}")),
    }
}

fn encode_direction(direction: Direction) -> &'static str {
    match direction {
        Direction::Inbound => "inbound",
        Direction::Outbound => "outbound",
    }
}
