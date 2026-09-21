//! What holds a body up, read from the ground the terrain is meshed from.
//!
//! Nature is a surface (docs/BRIEF.md): one ground per direction, no cave, no
//! overhang, nothing to be inside of. So a footing is the ground height, and
//! it is read from the generator rather than from a mesh of it, because a mesh
//! is filtered by how far the camera is and what a body stands on may not be.
//!
//! The shape here still carries a ceiling and a room, and `admits` still asks
//! about them. That is not dead weight: a build volume has an inside, and when
//! one is under the feet this reads its cells the way it reads the ground now.

use glam::DVec3;
use topology::BLOCK_M;
use worldgen::{Direction, Generator};

/// A rise a walker takes in its stride. One block; two is a wall, to be
/// jumped or flown (docs/ROADMAP.md).
pub const STEP_M: f64 = BLOCK_M;
/// A standing body, metres. What a roof has to clear.
pub const BODY_M: f64 = 1.8;
/// Metres between probes along a ray looking for ground.
const PROBE_M: f64 = BLOCK_M / 2.0;

/// What a body at one direction stands on, and what is over its head.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Footing {
    /// Top of the solid at or under the feet, metres over the datum sphere.
    /// `None` where there is nothing to stand on, so the body falls.
    pub floor_m: Option<f64>,
    /// Bottom of the solid over the feet, where there is one in reach. Always
    /// `None` over open nature, which has no roof.
    pub ceiling_m: Option<f64>,
}

impl Footing {
    /// A ground that is only ever a height, which is all of nature.
    pub fn solid(ground_m: f64) -> Footing {
        Footing {
            floor_m: Some(ground_m),
            ceiling_m: None,
        }
    }

    /// Metres of open air over the floor, unbounded under the sky.
    pub fn room_m(self) -> f64 {
        match (self.floor_m, self.ceiling_m) {
            (Some(floor_m), Some(ceiling_m)) => ceiling_m - floor_m,
            _ => f64::INFINITY,
        }
    }
}

/// The footing of one direction.
///
/// Collision is the ground in full detail, never the filtered one: what a body
/// stands on may not change with how far away the camera is. So the footprint
/// asked for is zero.
pub fn footing(generator: &Generator, direction: Direction, _feet_m: f64) -> Footing {
    Footing::solid(generator.sample_at(direction, 0.0).height_m)
}

/// Whether a walking body may go from one footing to another: a floor within
/// one step up, and room over it to stand.
///
/// A body already under a low roof keeps whatever room it had, so a place too
/// tight to stand in is never a place to be stuck in.
pub fn admits(from: Footing, to: Footing, feet_m: f64) -> bool {
    match to.floor_m {
        // Nothing within reach to stand on: a ledge, and walking off one is
        // allowed. Falling is the ground's answer, not a wall's.
        None => true,
        Some(floor_m) => floor_m <= feet_m + STEP_M && to.room_m() >= BODY_M.min(from.room_m()),
    }
}

/// How far a ray from `from` along `toward` (unit, world space) runs before it
/// goes under the ground, up to `reach_m`. What keeps a third person camera
/// out of the hill its body is standing against.
pub fn clear_run_m(generator: &Generator, from: DVec3, toward: DVec3, reach_m: f64) -> f64 {
    let radius_m = generator.sphere().radius_m();
    let mut at_m = PROBE_M;
    while at_m < reach_m {
        let at = from + toward * at_m;
        let direction = at.normalize_or(DVec3::Y).to_array();
        if at.length() - radius_m < generator.sample_at(direction, 0.0).height_m {
            return at_m - PROBE_M;
        }
        at_m += PROBE_M;
    }
    reach_m
}
