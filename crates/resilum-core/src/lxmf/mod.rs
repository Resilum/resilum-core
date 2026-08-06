//! Maps the FFI's LXMF message/event JSON onto `leviculum_lxmf` types. Pure
//! conversion — no node, no I/O — so it is testable without the messaging
//! backend it will feed once the std runtime can drive an `LxmfNode`.

pub mod poll;
pub mod send;

use leviculum_lxmf::Field;
use leviculum_lxmf::constants::{FIELD_CUSTOM_DATA, FIELD_CUSTOM_TYPE};
use serde_json::Value;

/// The app's `{custom_type, custom_data}` as LXMF custom fields — each value a
/// single msgpack value, as `Message::create` and the wire require.
fn encode_fields(custom_type: Option<&str>, custom_data: Option<&Value>) -> Vec<Field> {
    let mut fields = Vec::new();
    if let Some(t) = custom_type
        && let Ok(bytes) = rmp_serde::to_vec(t)
    {
        fields.push((FIELD_CUSTOM_TYPE, bytes));
    }
    if let Some(d) = custom_data
        && let Ok(bytes) = rmp_serde::to_vec(d)
    {
        fields.push((FIELD_CUSTOM_DATA, bytes));
    }
    fields
}

/// Inverse of [`encode_fields`]: recover `{custom_type, custom_data}` from an
/// LXMF message's fields, ignoring fields the app doesn't model.
fn decode_fields(fields: &[Field]) -> (Option<String>, Option<Value>) {
    let mut custom_type = None;
    let mut custom_data = None;
    for (id, bytes) in fields {
        match *id {
            FIELD_CUSTOM_TYPE => custom_type = rmp_serde::from_slice(bytes).ok(),
            FIELD_CUSTOM_DATA => custom_data = rmp_serde::from_slice(bytes).ok(),
            _ => {}
        }
    }
    (custom_type, custom_data)
}

#[cfg(test)]
mod tests;
