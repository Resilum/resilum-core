//! Render an `LxmfNodeEvent` as the FFI poll JSON: an inbound message or a
//! delivery-state update. Events the app doesn't surface return `None`.

use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::{DeliveryMethod, LxmfNodeEvent, Message};
use serde_json::{Value, json};

pub fn event_to_json(event: &LxmfNodeEvent) -> Option<String> {
    let value = match event {
        LxmfNodeEvent::MessageReceived(message) => message_value(message),
        LxmfNodeEvent::Submitted {
            message_id, method, ..
        } => delivery_value(message_id, submitted_state(*method)),
        LxmfNodeEvent::Delivered { message_id } => delivery_value(message_id, "delivered"),
        LxmfNodeEvent::DeliveryFailed { message_id, .. } => delivery_value(message_id, "failed"),
        _ => return None,
    };
    serde_json::to_string(&value).ok()
}

fn submitted_state(method: DeliveryMethod) -> &'static str {
    match method {
        DeliveryMethod::Propagated => "propagated",
        _ => "sent",
    }
}

fn delivery_value(message_id: &[u8; 32], state: &str) -> Value {
    json!({
        "type": "delivery",
        "message_id": HEXLOWER.encode(message_id),
        "state": state,
    })
}

fn message_value(message: &Message) -> Value {
    let (custom_type, custom_data) = super::decode_fields(&message.fields);
    json!({
        "type": "message",
        "source": HEXLOWER.encode(&message.source_hash),
        "message_id": HEXLOWER.encode(&message.message_id),
        "timestamp": message.timestamp,
        "title_b64": BASE64.encode(&message.title),
        "content_b64": BASE64.encode(&message.content),
        "fields": { "custom_type": custom_type, "custom_data": custom_data },
    })
}
