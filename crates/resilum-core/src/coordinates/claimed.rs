use serde::{Deserialize, Serialize};
use violin::Coord;

use super::Space;

const FURTHEST: f64 = 600.0;
const LEAST_ERROR: f64 = 0.01;
pub(super) const MOST_ERROR: f64 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Claimed {
    pub(super) position: [f64; 3],
    pub(super) height: f64,
    pub(super) error: f64,
}

impl Claimed {
    pub(super) fn of(coord: &Coord<Space>) -> Self {
        let raw = coord.raw_coord().as_ref();
        Self {
            position: [raw[0], raw[1], raw[2]],
            height: coord.height(),
            error: coord.error_estimate(),
        }
    }

    pub(super) fn believable(self) -> Option<Coord<Space>> {
        if !self.position.iter().all(|axis| axis.abs() <= FURTHEST) {
            return None;
        }
        if !(0.0..=FURTHEST).contains(&self.height) {
            return None;
        }
        if !self.error.is_finite() {
            return None;
        }
        let mut coord = Coord::<Space>::from(self.position);
        coord.set_height(self.height);
        coord.set_error_estimate(self.error.clamp(LEAST_ERROR, MOST_ERROR));
        Some(coord)
    }
}

#[cfg(test)]
mod tests;
