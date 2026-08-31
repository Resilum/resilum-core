mod defrag;

pub use defrag::{Arrived, Reassembly};

use super::spec;

#[must_use]
pub fn payload_per_fragment(bytes_one_write_carries: usize) -> usize {
    bytes_one_write_carries.saturating_sub(spec::FRAGMENT_HEADER_LEN)
}

#[must_use]
pub fn fragment(packet: &[u8], bytes_one_write_carries: usize) -> Vec<Vec<u8>> {
    let room = payload_per_fragment(bytes_one_write_carries);
    if room == 0 {
        return Vec::new();
    }
    let pieces: Vec<&[u8]> = if packet.is_empty() {
        vec![packet]
    } else {
        packet.chunks(room).collect()
    };
    let total = pieces.len();
    pieces
        .into_iter()
        .enumerate()
        .map(|(index, piece)| {
            let mut out = header(index, total).to_vec();
            out.extend_from_slice(piece);
            out
        })
        .collect()
}

fn header(index: usize, total: usize) -> [u8; spec::FRAGMENT_HEADER_LEN] {
    let kind = if index == 0 {
        spec::TYPE_START
    } else if index + 1 == total {
        spec::TYPE_END
    } else {
        spec::TYPE_CONTINUE
    };
    let seq = u16::try_from(index).unwrap_or(u16::MAX).to_be_bytes();
    let all = u16::try_from(total).unwrap_or(u16::MAX).to_be_bytes();
    [kind, seq[0], seq[1], all[0], all[1]]
}

#[cfg(test)]
mod tests;
