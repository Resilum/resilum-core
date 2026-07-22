//! Hex-encoded Identity helpers.

use leviculum_std::api::Identity;

pub fn identity(hex: &str) -> Result<Identity, String> {
    let bytes = decode(hex).ok_or_else(|| "not valid hex".to_string())?;
    Identity::from_public_key_bytes(&bytes).map_err(|e| format!("identity decode: {e:?}"))
}

fn decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}
