mod space;

use std::collections::HashMap;

use resilum_core::status::NodeStatus;

use super::paint::{a_mark, dimmed};
use space::{Span, Written, flat, put, reach_of, room_for};
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
    let held: Vec<(f64, f64)> = status
        .coordinates
        .peers
        .iter()
        .zip(&placed)
        .filter(|(peer, _)| !ways_to(status, &peer.identity_hash).is_empty())
        .map(|(_, at)| *at)
        .collect();
    let (x, y) = match reach_of(ours, &held) {
        Some(reach) => (Span::around(ours.0, reach), Span::around(ours.1, reach)),
        None => (
            Span::over(std::iter::once(ours.0).chain(placed.iter().map(|p| p.0))),
            Span::over(std::iter::once(ours.1).chain(placed.iter().map(|p| p.1))),
        ),
    };

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
        let Some(marked) = put(
            &mut cells,
            spot,
            Written {
                text: a_mark(&letter(nth).to_string(), quickest.as_deref()),
                wide: 1,
            },
        ) else {
            continue;
        };
        beside(
            &mut cells,
            marked,
            &format!(" {} ms", peer.estimated_rtt_ms),
        );
    }

    let mut drawn = vec![
        dimmed(&in_the_middle(&apart(y.high() - ours.1))),
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
        &apart(x.low - ours.0),
        &apart(y.low - ours.1),
        &apart(x.high() - ours.0),
    )));
    drawn
}

fn apart(seconds: f64) -> String {
    format!("{:+.0} ms", seconds * 1000.0)
}

fn beside(cells: &mut HashMap<(i64, i64), Written>, mark: (i64, i64), how_far: &str) {
    let wide = how_far.chars().count() as i64;
    let touching = [
        (mark.0 + 1, mark.1),
        (mark.0, mark.1 + 1),
        (mark.0 + 1, mark.1 + 1),
        (mark.0 + 1, mark.1 - 1),
        (mark.0, mark.1 - 1),
        (mark.0 - wide, mark.1),
        (mark.0 - wide, mark.1 + 1),
        (mark.0 - wide, mark.1 - 1),
    ];
    let written = Written {
        text: dimmed(how_far),
        wide,
    };
    for spot in touching {
        if room_for(cells, spot, wide) {
            cells.insert(spot, written);
            return;
        }
    }
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
