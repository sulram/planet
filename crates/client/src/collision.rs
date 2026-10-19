//! What holds a body up: the ground the terrain is meshed from, and the cells
//! of any volume under or over it.
//!
//! Nature is a surface (docs/BRIEF.md): one ground per direction, no cave, no
//! overhang, nothing to be inside of. So a footing over nature is the ground
//! height, read from the generator rather than from a mesh of it, because a
//! mesh is filtered by how far the camera is and what a body stands on may not
//! be. A volume has an inside, and its footing (`build`) carries a roof as
//! well, which is what `admits` asks about.

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
/// Times the stretch of a ray between the last probe over the ground and
/// the first under it is halved to find where it goes in: to a sixteenth
/// of a probe.
const HALVINGS: usize = 4;
/// Half the width of a standing body in address space, in blocks: how near
/// a wall it comes, and how far past the edge of a floor it still stands.
pub const BODY_HALF: f64 = 0.6;

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
    /// Open air: nothing under, nothing over.
    pub const OPEN: Footing = Footing {
        floor_m: None,
        ceiling_m: None,
    };

    /// A ground that is only ever a height, which is all of nature.
    pub fn solid(ground_m: f64) -> Footing {
        Footing {
            floor_m: Some(ground_m),
            ceiling_m: None,
        }
    }

    /// Both at once: the higher floor and the lower roof. What a body over
    /// the ground and a volume together stands on.
    pub fn with(self, other: Footing) -> Footing {
        let pick = |a: Option<f64>, b: Option<f64>, f: fn(f64, f64) -> f64| match (a, b) {
            (Some(a), Some(b)) => Some(f(a, b)),
            (a, b) => a.or(b),
        };
        Footing {
            floor_m: pick(self.floor_m, other.floor_m, f64::max),
            ceiling_m: pick(self.ceiling_m, other.ceiling_m, f64::min),
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
    let under = |at: DVec3| {
        let direction = at.normalize_or(DVec3::Y).to_array();
        at.length() - radius_m < generator.sample_at(direction, 0.0).height_m
    };
    ground_run_m(from, toward, reach_m, under).map_or(reach_m, |hit_m| (hit_m - PROBE_M).max(0.0))
}

/// How far a ray from `from` along `toward`, unit, runs before it goes into
/// whatever `under` says is below the ground, up to `reach_m`: `None` when
/// it stays over it that far. Probed every [`PROBE_M`], then halved in to
/// where it goes under.
pub fn ground_run_m(
    from: DVec3,
    toward: DVec3,
    reach_m: f64,
    under: impl Fn(DVec3) -> bool,
) -> Option<f64> {
    let steps = (reach_m / PROBE_M).floor().max(0.0) as usize;
    let step = (1..=steps).find(|&i| under(from + toward * (i as f64 * PROBE_M)))?;
    let (mut over_m, mut under_m) = ((step - 1) as f64 * PROBE_M, step as f64 * PROBE_M);
    for _ in 0..HALVINGS {
        let middle_m = (over_m + under_m) / 2.0;
        match under(from + toward * middle_m) {
            true => under_m = middle_m,
            false => over_m = middle_m,
        }
    }
    Some(under_m)
}
