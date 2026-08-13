//! Hex rendering for log lines and identifiers.

use std::fmt::Write;

/// The first eight bytes, which is how a hash reads in a log line.
pub fn head(bytes: &[u8]) -> String {
    encode(bytes.iter().take(8))
}

pub fn encode<'a>(bytes: impl Iterator<Item = &'a u8>) -> String {
    bytes.fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_stops_at_eight_bytes() {
        let bytes: Vec<u8> = (0..16).collect();
        assert_eq!(head(&bytes), "0001020304050607");
    }

    #[test]
    fn encode_pads_every_byte_to_two_digits() {
        assert_eq!(encode([0x00, 0x0f, 0xff].iter()), "000fff");
    }
}
