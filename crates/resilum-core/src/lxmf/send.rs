//! Parse the FFI send request into a signed `leviculum_lxmf::Message`.

use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::{DeliveryMethod, Message};
use leviculum_std::api::Identity;
use serde::Deserialize;
use serde_json::Value;

use super::encode_fields;

#[derive(Deserialize)]
struct SendRequest {
    dest: String,
    method: String,
    #[serde(default)]
    title_b64: Option<String>,
    #[serde(default)]
    content_b64: Option<String>,
    #[serde(default)]
    fields: Option<SendFields>,
}

#[derive(Deserialize)]
struct SendFields {
    #[serde(default)]
    custom_type: Option<String>,
    #[serde(default)]
    custom_data: Option<Value>,
}

/// Build and sign an LXMF message from the FFI JSON. `source`/`source_hash` and
/// `timestamp` come from the running node, not the request.
pub fn build_message(
    json: &str,
    source: &Identity,
    source_hash: [u8; 16],
    timestamp: f64,
) -> Result<Message, String> {
    let req: SendRequest =
        serde_json::from_str(json).map_err(|e| format!("bad message json: {e}"))?;
    let destination_hash = parse_hex16(&req.dest)?;
    let method = parse_method(&req.method)?;
    let title = decode_b64(req.title_b64.as_deref())?;
    let content = decode_b64(req.content_b64.as_deref())?;
    let (custom_type, custom_data) = match req.fields {
        Some(f) => (f.custom_type, f.custom_data),
        None => (None, None),
    };
    let fields = encode_fields(custom_type.as_deref(), custom_data.as_ref());
    Message::create(
        destination_hash,
        source_hash,
        source,
        timestamp,
        title,
        content,
        fields,
        method,
    )
    .map_err(|e| format!("lxmf message: {e:?}"))
}

fn parse_hex16(hex: &str) -> Result<[u8; 16], String> {
    let bytes = HEXLOWER
        .decode(hex.as_bytes())
        .map_err(|_| "dest is not hex".to_string())?;
    bytes
        .try_into()
        .map_err(|_| "dest is not a 16-byte destination".to_string())
}

fn parse_method(method: &str) -> Result<DeliveryMethod, String> {
    match method {
        "opportunistic" => Ok(DeliveryMethod::Opportunistic),
        "direct" => Ok(DeliveryMethod::Direct),
        "propagated" => Ok(DeliveryMethod::Propagated),
        other => Err(format!("unknown method: {other}")),
    }
}

fn decode_b64(value: Option<&str>) -> Result<Vec<u8>, String> {
    match value {
        Some(s) => BASE64
            .decode(s.as_bytes())
            .map_err(|_| "invalid base64".to_string()),
        None => Ok(Vec::new()),
    }
}
