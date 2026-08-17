//! Parse the FFI send request into a signed `leviculum_lxmf::Message`.

use data_encoding::{BASE64, HEXLOWER};
use leviculum_lxmf::{DeliveryMethod, Message};
use leviculum_std::api::Identity;
use serde::Deserialize;
use serde_json::Value;

use super::encode_fields;
use super::request::RequestError;

#[derive(Deserialize)]
struct SendRequest {
    destination: String,
    method: String,
    #[serde(default)]
    title_b64: Option<String>,
    #[serde(default)]
    content_b64: Option<String>,
    #[serde(default)]
    fields: Option<SendFields>,
    #[serde(default)]
    timestamp: Option<f64>,
}

#[derive(Deserialize)]
struct SendFields {
    #[serde(default)]
    custom_type: Option<String>,
    #[serde(default)]
    custom_data: Option<Value>,
}

/// Build and sign an LXMF message from the FFI JSON. `source`/`source_hash`
/// come from the running node, not the request. `timestamp` is the fallback
/// used when the request carries none of its own: a caller re-packing a
/// message it already sent supplies the original `timestamp` so the payload,
/// and therefore the message id, comes out identical on every attempt.
pub fn build_message(
    json: &str,
    source: &Identity,
    source_hash: [u8; 16],
    timestamp: f64,
) -> Result<Message, RequestError> {
    let req: SendRequest =
        serde_json::from_str(json).map_err(|e| RequestError::Json(e.to_string()))?;
    let destination_hash = parse_hex16(&req.destination)?;
    let method = parse_method(&req.method)?;
    let title = decode_b64(req.title_b64.as_deref(), "title_b64")?;
    let content = decode_b64(req.content_b64.as_deref(), "content_b64")?;
    let (custom_type, custom_data) = match req.fields {
        Some(f) => (f.custom_type, f.custom_data),
        None => (None, None),
    };
    let fields = encode_fields(custom_type.as_deref(), custom_data.as_ref());
    Message::create(
        destination_hash,
        source_hash,
        source,
        req.timestamp.unwrap_or(timestamp),
        title,
        content,
        fields,
        method,
    )
    // `{e:?}` and not `{e}`: the upstream error is `Debug`-only.
    .map_err(|e| RequestError::Signing(format!("{e:?}")))
}

fn parse_hex16(hex: &str) -> Result<[u8; 16], RequestError> {
    let refuse = || RequestError::Hex {
        field: "destination",
    };
    let bytes = HEXLOWER.decode(hex.as_bytes()).map_err(|_| refuse())?;
    bytes.try_into().map_err(|_| refuse())
}

pub(super) fn parse_method(method: &str) -> Result<DeliveryMethod, RequestError> {
    match method {
        "opportunistic" => Ok(DeliveryMethod::Opportunistic),
        "direct" => Ok(DeliveryMethod::Direct),
        "propagated" => Ok(DeliveryMethod::Propagated),
        other => Err(RequestError::UnknownMethod(other.to_owned())),
    }
}

fn decode_b64(value: Option<&str>, field: &'static str) -> Result<Vec<u8>, RequestError> {
    match value {
        Some(s) => BASE64
            .decode(s.as_bytes())
            .map_err(|_| RequestError::Base64 { field }),
        None => Ok(Vec::new()),
    }
}
