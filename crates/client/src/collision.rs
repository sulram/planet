//! What holds a body up, read from the density field.
//!
//! A height has one ground per column, so a heightfield walker stands on the
//! roof of every cave it crosses. The ground is a volume now, and what a body
//! stands on is the top of the solid under its feet: the surface out in the
//! open, the cave's own floor inside one.
//!
//! Collision reads the field, not a mesh of it. The field is what the mesh is
//! made from, a column of it is worked out once (`Generator::column`), and a
//! body needs one column where the renderer needs a patch of them. A collision
//! mesh would be a cache of exactly this, and it is not one until the field
//! stops being the whole truth: the day a stored chunk carries an edit, this
//! reads `chunk(addr)` like everything else, and that is when it earns a
//! representation of its own.

use glam::DVec3;
use topology::{BLOCK_M, RADIUS_M};
use worldgen::{Direction, Generator};

/// A rise a walker takes in its stride. One block; two is a wall, to be
/// jumped or flown (docs/ROADMAP.md).
pub const STEP_M: f64 = BLOCK_M;
/// A standing body, metres. What a roof has to clear.
pub const BODY_M: f64 = 1.8;

/// Metres over the feet a footing looks for rock: a body and the step it may
/// be lifted by, which is all the rock a walker can find itself inside of.
const REACH_UP_M: f64 = BODY_M + STEP_M;
/// And under them: further than anything falling near the ground crosses in
/// one frame.
const REACH_DOWN_M: f64 = 6.0;
/// Metres between probes going down a column. Half a cell of the volume,
/// because a slab one cell thick is drawn and a probe as coarse as one would
/// straddle it: a body would fall through a ledge it can see. Thinner than a
/// cell the mesher has nothing to draw either.
const PROBE_M: f64 = BLOCK_M / 2.0;
/// Bisections that turn a bracket into a surface: half a block to a millimetre.
const REFINE: u32 = 8;

/// What a body at one direction stands on, and what is over its head.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Footing {
    /// Top of the solid at or under the feet, metres over the datum sphere.
    /// `None` where the column is open further down than we look: nothing to
    /// stand on, so the body falls.
    pub floor_m: Option<f64>,
    /// Bottom of the solid over the feet, where there is one in reach.
    pub ceiling_m: Option<f64>,
}

impl Footing {
    /// A ground that is only ever a height: the moon, and any world on a
    /// generator version frozen without caves.
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

/// The footing of one direction, for feet at `feet_m` over the datum sphere.
pub fn footing(generator: &Generator, direction: Direction, feet_m: f64) -> Footing {
    // Collision is the ground in full detail, never the filtered one: what a
    // body stands on may not change with how far away the camera is.
    let column = generator.column(direction, 0.0);
    let ground_m = column.ground().height_m;
    if column.solid() {
        return Footing::solid(ground_m);
    }

    // Rock at knee height means the body is in rock rather than on it: a
    // wall it must not walk into, or rock it has to be let out of. The
    // ground is solid and above the top of that rock, so it is the floor in
    // both cases, and a wall taller than a footing sees stops a body like
    // any other wall. A body standing exactly on the surface reads the same
    // way, and the surface is indeed its floor.
    let knee_m = (feet_m + STEP_M).min(ground_m);
    if column.density_m(knee_m) >= 0.0 {
        return Footing::solid(ground_m);
    }

    // Nothing over the ground is rock, so that is where the search starts.
    let top_m = (feet_m + REACH_UP_M).min(ground_m);
    let bottom_m = feet_m - REACH_DOWN_M;
    let mut ceiling_m = None;
    let mut high_m = top_m;
    let mut high = column.density_m(high_m);
    while high_m > bottom_m {
        let low_m = (high_m - PROBE_M).max(bottom_m);
        let low = column.density_m(low_m);
        if (high < 0.0) != (low < 0.0) {
            let surface_m = refine(&column, low_m, high_m);
            if low >= 0.0 {
                // Air over rock: the floor, and the search is done.
                return Footing {
                    floor_m: Some(surface_m),
                    ceiling_m,
                };
            }
            // Rock over air: a roof, if it is over the feet. They come down
            // the column in order, so the last one kept is the lowest.
            if surface_m > feet_m {
                ceiling_m = Some(surface_m);
            }
        }
        (high_m, high) = (low_m, low);
    }
    // Air the whole way down: a ledge, a shaft, or a chamber taller than a
    // footing looks. Nothing to stand on, so the body falls. Rock the whole
    // way down means the knee found a pocket thinner than a probe, and the
    // rock is the safer answer of the two.
    Footing {
        floor_m: (high >= 0.0).then_some(ground_m),
        ceiling_m,
    }
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

/// How far a ray from `from` along `toward` (unit, world space) runs before
/// it enters rock, up to `reach_m`. Planet only: a height has nothing to
/// enter. What keeps a third person camera in the cave its body walked into.
pub fn clear_run_m(generator: &Generator, from: DVec3, toward: DVec3, reach_m: f64) -> f64 {
    let mut at_m = PROBE_M;
    while at_m < reach_m {
        let at = from + toward * at_m;
        let height_m = at.length() - RADIUS_M;
        if generator.density_m(at.normalize_or(DVec3::Y).to_array(), height_m, 0.0) > 0.0 {
            return at_m - PROBE_M;
        }
        at_m += PROBE_M;
    }
    reach_m
}

/// The height between two brackets where the density crosses zero. The field
/// is not a straight line, so this is bisection and not one division.
fn refine(column: &worldgen::Column, low_m: f64, high_m: f64) -> f64 {
    let (mut low_m, mut high_m) = (low_m, high_m);
    for _ in 0..REFINE {
        let middle_m = (low_m + high_m) * 0.5;
        if column.density_m(middle_m) >= 0.0 {
            low_m = middle_m;
        } else {
            high_m = middle_m;
        }
    }
    (low_m + high_m) * 0.5
}
