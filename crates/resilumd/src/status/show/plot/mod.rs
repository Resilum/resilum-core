mod space;

use std::collections::HashMap;

use resilum_core::status::NodeStatus;

use super::paint::{a_mark, dimmed};
use space::{Span, Written, flat, put};
pub use space::{letter, quickest_way_to, ways_to};

const WIDE: i64 = 58;
const TALL: i64 = 15;

pub const US: char = '◉';

pub fn of(status: &NodeStatus) -> Vec<String> {
    let ours = flat(status.coordinates.ours.position());
    let placed: Vec<(f64, f64)> = status
        .coordinates
        .peers
        .iter()
        .map(|peer| flat(peer.at.position()))
        .collect();
    let x = Span::over(std::iter::once(ours.0).chain(placed.iter().map(|p| p.0)));
    let y = Span::over(std::iter::once(ours.1).chain(placed.iter().map(|p| p.1)));

    let mut cells: HashMap<(i64, i64), Written> = HashMap::new();
    cells.insert(
        (x.cell(ours.0, WIDE), y.row(ours.1, TALL)),
        Written {
            text: US.to_string(),
            wide: 1,
        },
    );
    for (nth, at) in placed.iter().enumerate() {
        let peer = &status.coordinates.peers[nth];
        let spot = (x.cell(at.0, WIDE), y.row(at.1, TALL));
        let quickest = quickest_way_to(status, &peer.identity_hash);
        let how_far = format!(" {} ms", peer.estimated_rtt_ms);
        put(
            &mut cells,
            spot,
            Written {
                text: format!(
                    "{}{}",
                    a_mark(&letter(nth).to_string(), quickest.as_deref()),
                    dimmed(&how_far)
                ),
                wide: 1 + how_far.chars().count() as i64,
            },
        );
    }

    let mut drawn = vec![
        dimmed(&in_the_middle(&format!("y {:+.3}", y.high()))),
        dimmed(&format!("┌{}┐", "─".repeat(WIDE as usize))),
    ];
    for row in 0..TALL {
        let mut inside = String::new();
        let mut column = 0;
        while column < WIDE {
            match cells.get(&(column, row)) {
                None => {
                    inside.push(' ');
                    column += 1;
                }
                Some(written) => {
                    inside.push_str(&written.text);
                    column += written.wide;
                }
            }
        }
        drawn.push(format!("{}{inside}{}", dimmed("│"), dimmed("│")));
    }
    drawn.push(dimmed(&format!("└{}┘", "─".repeat(WIDE as usize))));
    drawn.push(dimmed(&across(
        &format!("x {:+.3}", x.low),
        &format!("y {:+.3}", y.low),
        &format!("x {:+.3}", x.high()),
    )));
    drawn
}

fn in_the_middle(text: &str) -> String {
    let room = WIDE as usize + 2;
    let before = room.saturating_sub(text.chars().count()) / 2;
    format!("{}{text}", " ".repeat(before))
}

fn across(left: &str, middle: &str, right: &str) -> String {
    let room = WIDE as usize + 2;
    let mut line = in_the_middle(middle);
    line.replace_range(..left.chars().count(), left);
    let from = room.saturating_sub(right.chars().count());
    if line.chars().count() < from {
        line.push_str(&" ".repeat(from - line.chars().count()));
    }
    line.truncate(from);
    line.push_str(right);
    line
}

#[cfg(test)]
mod tests;
