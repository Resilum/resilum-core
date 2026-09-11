use std::collections::HashMap;

use resilum_core::status::NodeStatus;

use super::{TALL, WIDE};

pub fn letter(nth: usize) -> char {
    char::from(b'a' + u8::try_from(nth % 26).unwrap_or(0))
}

pub struct Written {
    pub text: String,
    pub wide: i64,
}

pub const NUDGE: [(i64, i64); 5] = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)];

pub fn put(
    cells: &mut HashMap<(i64, i64), Written>,
    at: (i64, i64),
    written: Written,
) -> Option<(i64, i64)> {
    let wide = written.wide;
    let free = NUDGE
        .iter()
        .map(|nudge| (at.0 + nudge.0, at.1 + nudge.1))
        .map(|spot| ((spot.0).min(WIDE - wide).max(0), spot.1))
        .find(|spot| room_for(cells, *spot, wide))?;
    cells.insert(free, written);
    Some(free)
}

pub fn room_for(cells: &HashMap<(i64, i64), Written>, at: (i64, i64), wide: i64) -> bool {
    let inside = (0..TALL).contains(&at.1) && at.0 >= 0 && at.0 + wide <= WIDE;
    inside && (0..wide).all(|step| !taken(cells, (at.0 + step, at.1)))
}

fn taken(cells: &HashMap<(i64, i64), Written>, at: (i64, i64)) -> bool {
    cells
        .iter()
        .any(|(spot, written)| spot.1 == at.1 && spot.0 <= at.0 && at.0 < spot.0 + written.wide)
}

pub fn flat(position: [f64; 3]) -> (f64, f64) {
    (position[0], position[1])
}

const MOST_OF_THEM: f64 = 0.9;

pub fn reach_of(ours: (f64, f64), held: &[(f64, f64)]) -> Option<f64> {
    let mut apart: Vec<f64> = held
        .iter()
        .map(|at| (at.0 - ours.0).hypot(at.1 - ours.1))
        .filter(|far| far.is_finite())
        .collect();
    apart.sort_by(f64::total_cmp);
    let nth = ((apart.len() as f64 - 1.0) * MOST_OF_THEM).round() as usize;
    apart
        .get(nth)
        .copied()
        .filter(|reach| *reach > f64::EPSILON)
}

pub fn ways_to(status: &NodeStatus, peer: &str) -> Vec<String> {
    let mut over: Vec<String> = status
        .links
        .iter()
        .filter(|link| link.identity_hash == peer)
        .map(|link| link.transport.clone())
        .collect();
    over.sort_unstable();
    over.dedup();
    over
}

pub fn quickest_way_to(status: &NodeStatus, peer: &str) -> Option<String> {
    status
        .links
        .iter()
        .filter(|link| link.identity_hash == peer)
        .min_by_key(|link| link.estimated_rtt_ms.unwrap_or(u128::MAX))
        .map(|link| link.transport.clone())
}

pub struct Span {
    pub low: f64,
    reach: f64,
}

impl Span {
    pub fn around(centre: f64, reach: f64) -> Self {
        Self {
            low: centre - reach,
            reach: reach * 2.0,
        }
    }

    pub fn over(values: impl Iterator<Item = f64>) -> Self {
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
        for v in values {
            low = low.min(v);
            high = high.max(v);
        }
        let reach = high - low;
        Self {
            low,
            reach: if reach > f64::EPSILON { reach } else { 0.0 },
        }
    }

    pub fn high(&self) -> f64 {
        self.low + self.reach
    }

    pub fn cell(&self, value: f64, cells: i64) -> i64 {
        let a_span_of_nothing = self.reach == 0.0;
        if a_span_of_nothing {
            return cells / 2;
        }
        let share = (value - self.low) / self.reach;
        ((share * (cells - 1) as f64).round() as i64).clamp(0, cells - 1)
    }

    pub fn row(&self, value: f64, rows: i64) -> i64 {
        rows - 1 - self.cell(value, rows)
    }
}
