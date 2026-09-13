//! Hex rendering for log lines and identifiers.

/// The first eight bytes, which is how a hash reads in a log line.
pub fn head(bytes: &[u8]) -> String {
    encode(bytes.iter().take(8))
}

pub fn encode<'a>(bytes: impl Iterator<Item = &'a u8>) -> String {
    bytes
        .flat_map(|byte| [digit(byte >> 4), digit(byte & 0x0f)])
        .collect()
}

fn digit(nibble: u8) -> char {
    char::from(match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + nibble - 10,
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
