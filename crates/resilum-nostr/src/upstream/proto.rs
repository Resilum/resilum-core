//! Nostr relay protocol: frames we send and parse from the relay.

use crate::event::Event;
use serde_json::{Value, json};

#[derive(Debug)]
pub enum Incoming {
    Event {
        event: Box<Event>,
    },
    Verdict {
        event_id: String,
        accepted: bool,
        message: String,
    },
    EndOfStored {
        subscription: String,
    },
    /// How a relay says a `REQ` was refused: `auth-required`, `rate-limited`,
    /// `unsupported filter`.
    Closed {
        subscription: String,
        message: String,
    },
    Notice(String),
}

/// Returns `None` for frames we do not model, allowing the relay to send
/// future types without being fatal.
///
/// NIP-42 `AUTH` is among those: answering a challenge needs a relay-facing
/// identity this bridge does not have, and one that would tie every
/// subscriber it carries to the operator. A relay that wants it refuses each
/// `REQ` with `CLOSED … auth-required`, which is what says so in the log.
pub fn parse_incoming(text: &str) -> Option<Incoming> {
    let Value::Array(array) = serde_json::from_str::<Value>(text).ok()? else {
        return None;
    };
    // Consumed rather than indexed: every field is moved out of the parsed
    // frame, and the event body is the largest thing this crate handles.
    let mut fields = array.into_iter();
    let tag = fields.next()?;

    match tag.as_str()? {
        "EVENT" => {
            // Read past, not kept: the id names the batch, not a destination
            // — the `p` tag is what routes the event.
            text_field(&mut fields)?;
            let event = serde_json::from_value::<Event>(fields.next()?).ok()?;
            Some(Incoming::Event {
                event: Box::new(event),
            })
        }
        "OK" => {
            let event_id = text_field(&mut fields)?;
            let accepted = fields.next()?.as_bool()?;
            let message = text_field(&mut fields)?;
            Some(Incoming::Verdict {
                event_id,
                accepted,
                message,
            })
        }
        "EOSE" => Some(Incoming::EndOfStored {
            subscription: text_field(&mut fields)?,
        }),
        "CLOSED" => Some(Incoming::Closed {
            subscription: text_field(&mut fields)?,
            message: text_field(&mut fields)?,
        }),
        "NOTICE" => Some(Incoming::Notice(text_field(&mut fields)?)),
        _ => None,
    }
}

fn text_field(fields: &mut impl Iterator<Item = Value>) -> Option<String> {
    match fields.next()? {
        Value::String(text) => Some(text),
        _ => None,
    }
}

pub fn publish_frame(event: &Event) -> Option<String> {
    let event_value = serde_json::to_value(event).ok()?;
    Some(json!(["EVENT", event_value]).to_string())
}

/// One subscriber's terms. Several ride in a single `REQ`, so each keeps the
/// `since` it earned: a filter merged across subscribers would have to pick
/// one mark for the group, refetching for some or skipping messages for the
/// rest.
#[derive(Debug)]
pub struct Filter {
    pub kinds: Vec<u32>,
    pub subscriber_pubkey_hex: String,
    pub since: Option<i64>,
}

/// NIP-01 returns an event matching any of the filters on one subscription,
/// so the id names the batch and not a destination.
pub fn request_frame(subscription: &str, filters: &[Filter]) -> String {
    let mut frame = vec![json!("REQ"), json!(subscription)];
    frame.extend(filters.iter().map(filter_json));
    Value::Array(frame).to_string()
}

/// `since` is omitted rather than null or zero: a relay reads either of those
/// as a filter and we would lose the stored history the subscriber is owed.
fn filter_json(filter: &Filter) -> Value {
    let mut value = json!({
        "kinds": filter.kinds,
        "#p": [filter.subscriber_pubkey_hex],
    });

    if let Some(since) = filter.since {
        value["since"] = json!(since);
    }

    value
}

#[cfg(test)]
mod tests;
