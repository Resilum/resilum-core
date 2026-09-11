use serde::{Deserialize, Serialize};
use violin::Coord;

use super::Space;

const FURTHEST: f64 = 600.0;
const LEAST_ERROR: f64 = 0.01;
pub(super) const MOST_ERROR: f64 = 1.5;
/// No way into a mesh is a quarter of a second long, and `violin` grows a
/// height by dividing by the gap between two coordinates — which on a mesh
/// whose peers answer in milliseconds is small enough to run away with it.
pub(super) const MOST_LAST_MILE: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Claimed {
    pub(super) position: [f64; 3],
    pub(super) height: f64,
    pub(super) error: f64,
}

impl Claimed {
    #[must_use]
    pub fn position(self) -> [f64; 3] {
        self.position
    }

    #[must_use]
    pub fn error(self) -> f64 {
        self.error
    }

    #[must_use]
    pub fn height(self) -> f64 {
        self.height
    }

    pub(super) fn of(coord: &Coord<Space>) -> Self {
        let raw = coord.raw_coord().as_ref();
        Self {
            position: [raw[0], raw[1], raw[2]],
            height: coord.height().clamp(0.0, MOST_LAST_MILE),
            error: coord.error_estimate().clamp(LEAST_ERROR, MOST_ERROR),
        }
    }

    pub(super) fn believable(self) -> Option<Coord<Space>> {
        if !self.position.iter().all(|axis| axis.abs() <= FURTHEST) {
            return None;
        }
        if !self.height.is_finite() || !self.error.is_finite() {
            return None;
        }
        let mut coord = Coord::<Space>::from(self.position);
        coord.set_height(self.height.clamp(0.0, MOST_LAST_MILE));
        coord.set_error_estimate(self.error.clamp(LEAST_ERROR, MOST_ERROR));
        Some(coord)
    }
}

#[cfg(test)]
mod tests;
