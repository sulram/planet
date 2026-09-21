//! Generator version 3. Frozen once a release ships it.
//!
//! What changed from v2, and why:
//! - **The shape is a source.** Everything below reads two numbers, `land` and
//!   `ranges` ([`Shape`]), and knows nothing about where they came from. A
//!   world with only a seed gets them from its tectonic plates; a world whose
//!   recipe names a [`Field`] gets them from that baked ground. One body, one
//!   read path, one set of tuning, two kinds of world.
//! - **Ranges are plate edges.** v2 drew them from a blob of noise, which gave
//!   patches. Now a belt runs along a boundary, a trench faces the plate that
//!   dives under, and an arc of volcanoes faces the other way.
//! - **The land carries its own height.** v2 let one term of noise decide how
//!   high a continent sat; here `land` itself lifts it, so a plateau reads as
//!   a plateau and not as a plain with mountains dropped on it.
//!
//! Above the source's own resolution the ground is the source's. Below it,
//! the noise takes over, faded by `footprint_m` exactly as in v2: a field is
//! the low frequencies of a real body, never the whole of it, and at our
//! radius there is a great deal of world under one of its texels.

use crate::field::Field;
use crate::noise::{band, fbm, simplex_d, smoothstep};
use crate::plates::{Plates, Shape};
use crate::{Direction, Material, Recipe, Sample};
use topology::QuadSphere;

const MOUNTAIN: u64 = 0x4_1000;
const HILL: u64 = 0x4_2000;
const DETAIL: u64 = 0x4_3000;
const MOISTURE: u64 = 0x4_4000;
const SEABED: u64 = 0x4_5000;
const CAVE: u64 = 0x4_6000;

/// Radius the wavelengths below are quoted against, metres. A constant of the
/// generator, not of the topology: changing it would change the terrain.
const RADIUS_M: f64 = 20_860.0;

/// How wide a cave system's turns are, metres. Also the wavelength the
/// footprint fades it against.
const CAVE_M: f64 = 70.0;
/// How near a ridged sum's crest counts as inside a tunnel, where a world has
/// no cave country at all. Nothing reaches it.
const CAVE_CREST: f64 = 0.94;
/// How far cave country lowers that bar. One threshold could not do both jobs:
/// a shade lower and the surface is fringed with mouths everywhere, a shade
/// higher and a world has no caves at all. So how *common* a cave is and how
/// *wide* it is became two knobs, and the first is a region.
const CAVE_COUNTRY: f64 = 0.085;
/// Cave country, in cycles per radius: limestone here, granite there.
const CAVE_REGION: f64 = 9.0;
/// Metres the wall of a tunnel stands from its middle, at the widest. What
/// decides the shape is `CAVE_CREST`; this only says how sharply the wall
/// arrives, which is what a mesher places its vertex against.
const CAVE_WIDTH_M: f64 = 60.0;
/// How deep a cave may reach, metres under the ground. Below this is the
/// bedrock at the floor of the build band, which nobody digs through.
const CAVE_FLOOR_M: f64 = 120.0;

/// How hard accumulated slope damps the finer octaves.
const EROSION: f64 = 0.06;
/// Amplitude kept from one octave to the next.
const GAIN: f64 = 0.58;

/// Metres of source elevation that count as one unit of inland `land`. A
/// coastline is zero on both sides, so a field's shore lands exactly where the
/// body it was baked from has one.
const LAND_M: f64 = 2_500.0;
/// Metres of source elevation that count as fully lifted ground.
const LIFT_M: f64 = 3_000.0;
/// The footprints the broad channels of a field are read at, metres.
///
/// Our planet is a three hundredth of the body a field is baked from, so its
/// slopes are three hundred times what they were. Read at the texel, a coast
/// range becomes a wall and a foothill becomes a needle. Lift and ranges are
/// therefore read broad, and the relief inside a belt is the generator's own,
/// which is tuned to stand up and be walked on.
///
/// The two differ because they measure different things. Elevation is already
/// smooth, and the edge of a plateau is worth keeping, so lift is read close
/// in. Ruggedness is the spread inside a texel, and asking it where a belt is
/// only answers over a belt's whole width.
const LIFT_FOOTPRINT_M: f64 = 250.0;
const BELT_FOOTPRINT_M: f64 = 1_000.0;
/// The same for the sea, where a body's range is far wider. The abyssal plain
/// then arrives at the `land` the sea floor profile below expects.
const SEA_M: f64 = 10_000.0;
/// Local relief on the source body where a belt starts, and where it is as
/// mountainous as a belt gets, metres. Measured against the ground Earth
/// actually has, at the scale below: the Amazon and the Sahara read zero,
/// the Rockies and the Andes a third, the Alps all of it.
const BELT_FLOOR_M: f64 = 60.0;
const BELT_M: f64 = 600.0;
/// As far inland as the body below is willing to read.
const INLAND_MAX: f64 = 1.6;
/// Metres of *broad* source elevation over which the land's own relief fades
/// in from the coast, and metres of the mesh's own elevation over which it is
/// pinned to zero at the shore.
const INLAND_M: f64 = 550.0;
const SHORE_M: f64 = 40.0;

/// A fractal sum whose octaves are damped by the slope gathered so far (after
/// Quilez), and faded by `band` below the sampling footprint. About `-1..=1`.
/// Unchanged from v2: what v3 changes is where the shape comes from, not how
/// a mountain weathers.
fn eroded(
    seed: u64,
    d: Direction,
    frequency: f64,
    octaves: u32,
    sharpness: f64,
    footprint: f64,
) -> f64 {
    let (mut sum, mut amplitude, mut f) = (0.0, 1.0, frequency);
    let mut slope = [0.0f64; 3];
    for octave in 0..octaves {
        let weight = band(1.0 / f, footprint);
        if weight <= 0.0 {
            break;
        }
        let (n, g) = simplex_d(
            seed.wrapping_add(u64::from(octave)),
            [d[0] * f, d[1] * f, d[2] * f],
        );
        let fold = if octave < 2 { 0.0 } else { sharpness };
        let soft = libm::sqrt(n * n + 0.04);
        let value = (1.0 - fold) * n + fold * (1.0 - 2.0 * soft);
        let value_slope = (1.0 - fold) - fold * 2.0 * n / soft;
        for axis in 0..3 {
            slope[axis] += g[axis] * value_slope;
        }
        let steepness = slope[0] * slope[0] + slope[1] * slope[1] + slope[2] * slope[2];
        sum += weight * amplitude * value / (1.0 + EROSION * steepness);
        amplitude *= GAIN;
        f *= 2.0;
    }
    sum / total_amplitude(octaves)
}

fn total_amplitude(octaves: u32) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    for _ in 0..octaves {
        total += amplitude;
        amplitude *= GAIN;
    }
    total
}

/// Plain fractal sum with the same band limit.
fn filtered(seed: u64, d: Direction, frequency: f64, octaves: u32, footprint: f64) -> f64 {
    let (mut sum, mut amplitude, mut f) = (0.0, 1.0, frequency);
    for octave in 0..octaves {
        let weight = band(1.0 / f, footprint);
        if weight <= 0.0 {
            break;
        }
        let p = [d[0] * f, d[1] * f, d[2] * f];
        sum += weight * amplitude * simplex_d(seed.wrapping_add(u64::from(octave)), p).0;
        amplitude *= GAIN;
        f *= 2.0;
    }
    sum / total_amplitude(octaves)
}

/// A baked body read as crust.
///
/// The elevation is not used as a height: at 1/305 of the body it was baked
/// from, honest elevations would make the planet a billiard ball, and honest
/// exaggeration would make every slope a wall. What survives the change of
/// scale is the *shape*: the coastline, which stays exactly where it is
/// because zero maps to zero, and where the ground is rough, which is where
/// a belt belongs. The relief itself is the generator's, and `relief_m` sets
/// how tall it grows.
fn field_shape(
    recipe: &Recipe,
    sphere: QuadSphere,
    field: &Field,
    d: Direction,
    footprint_m: f64,
) -> Shape {
    let params = &recipe.params;
    let ground = field.sample(sphere, d, footprint_m);
    // Where the shore is, is a choice: the body's own sea level, or higher,
    // or lower.
    let elevation_m = ground.elevation_m - params.sea_level_m;
    let land = if elevation_m < 0.0 {
        // Depth follows a curve, not a proportion. At our scale a proportion
        // leaves every strait and shelf sea under a metre of water; the curve
        // spends the depth near the shore and still reaches `ocean_depth_m`
        // out on the plain.
        -libm::pow(-elevation_m / SEA_M, params.sea_curve)
    } else {
        (elevation_m / LAND_M).min(INLAND_MAX)
    };
    // Never finer than the mesh that asked: a coarse patch pays for the level
    // it is already reading, not for a finer one.
    let lift = field.sample(sphere, d, footprint_m.max(LIFT_FOOTPRINT_M));
    let belt = field.sample(sphere, d, footprint_m.max(BELT_FOOTPRINT_M));
    Shape {
        land,
        lift: smoothstep(0.0, LIFT_M, lift.elevation_m),
        ranges: smoothstep(BELT_FLOOR_M, BELT_M, belt.ruggedness_m),
        // Broad, for the reason written on `Shape::inland`, and off the belt
        // sample because this gate multiplies more than either of the others.
        // The fine factor is there because a broad sample averages land with
        // sea and does not know exactly where the line is: it pins the gate to
        // zero at the shore the mesh actually draws, so no relief ever stands
        // out of the water. Near the shore the broad factor is itself near
        // zero, so the fine one's steepness never reaches the relief.
        inland: smoothstep(0.0, INLAND_M, belt.elevation_m - params.sea_level_m)
            * smoothstep(0.0, SHORE_M, elevation_m),
    }
}

pub fn sample(
    recipe: &Recipe,
    sphere: QuadSphere,
    plates: &Plates,
    field: Option<&Field>,
    d: Direction,
    footprint_m: f64,
) -> Sample {
    let seed = recipe.seed;
    let params = &recipe.params;
    let footprint = footprint_m / RADIUS_M;
    let scale = params.continent_scale;

    let shape = match field {
        Some(field) => field_shape(recipe, sphere, field, d, footprint_m),
        None => plates.shape(seed, d, params.sea_share),
    };
    let Shape {
        land,
        lift,
        ranges,
        inland,
    } = shape;

    let height_m = if land < 0.0 {
        let sea = -land;
        // Shelf (shallow, wide), slope, then the plain.
        let shelf = smoothstep(0.0, 0.06, sea) * 0.06;
        let deep = smoothstep(0.05, 0.40, sea) * 0.94;
        let relief = filtered(seed ^ SEABED, d, scale * 10.0, 6, footprint);
        // A ridge is rough ground under water: the same signal that raises a
        // cordillera on land raises seamounts here.
        let seamounts =
            ranges * (0.5 + 0.5 * eroded(seed ^ SEABED ^ 1, d, scale * 5.0, 6, 0.5, footprint));
        let ripples = filtered(seed ^ DETAIL, d, 900.0, 5, footprint);
        -params.ocean_depth_m * (shelf + deep) * (1.0 - 0.55 * seamounts * deep)
            + deep * 40.0 * relief
            + smoothstep(0.0, 0.02, sea) * 0.6 * ripples
            - 0.4
    } else {
        // Ease in from the coast so beaches are wide and flat. The gate is
        // the shape's, not a smoothstep of `land`: read at the mesh's own
        // resolution it makes every metre of relief follow the source's
        // gradient, and a coast range comes out as a cliff (58).
        let coast = smoothstep(0.0, 0.05, land);
        let mountains = 0.5 + 0.5 * eroded(seed ^ MOUNTAIN, d, scale * 5.0, 12, 0.9, footprint);
        let hills = filtered(seed ^ HILL, d, scale * 30.0, 8, footprint);
        let detail = filtered(seed ^ DETAIL, d, 1800.0, 5, footprint);
        params.relief_m * inland * (0.03 + 0.30 * lift + 0.80 * ranges * mountains)
            + inland * (1.0 - 0.6 * ranges) * 22.0 * hills
            + coast * 0.5 * detail
            + 0.6 * coast
    };

    Sample {
        height_m,
        material: material(recipe, d, height_m),
    }
}

fn material(recipe: &Recipe, d: Direction, height_m: f64) -> Material {
    let seed = recipe.seed;
    let relief = recipe.params.relief_m;
    if height_m < -6.0 {
        return Material::Seabed;
    }
    if height_m < 2.0 {
        return Material::Sand;
    }
    let wobble = fbm(
        seed ^ DETAIL,
        [d[0] * 90.0, d[1] * 90.0, d[2] * 90.0],
        3,
        0.5,
    );
    let h = height_m + wobble * 0.06 * relief;
    let cold = libm::fabs(d[1]) + h / relief * 0.9;
    if cold > 0.93 {
        return Material::Snow;
    }
    if h > 0.45 * relief {
        return Material::Rock;
    }
    let moisture = fbm(
        seed ^ MOISTURE,
        [d[0] * 5.0, d[1] * 5.0, d[2] * 5.0],
        4,
        0.5,
    );
    if moisture > 0.05 {
        Material::Forest
    } else {
        Material::Grass
    }
}

/// Everything a cave needs of a direction that does not change with height.
///
/// A volume asks for tens of samples up one line, and the rock under that line
/// is the same for all of them. Working it out per cell is the difference
/// between a volume that fits in a frame and one that does not, and most
/// columns turn out to want no cave at all, which is then free.
#[derive(Clone, Copy, Debug)]
pub struct Column {
    seed: u64,
    ground_m: f64,
    /// How near a crest counts as inside a tunnel here. `1` is unreachable.
    crest: f64,
    /// The share of a cave this column may have at all, before depth.
    allowed: f64,
}

impl Column {
    /// True when no height in this column can hold a cave, so the ground is
    /// the ground and nothing else need be asked.
    pub fn solid(&self) -> bool {
        self.crest >= 1.0
    }
}

pub fn column(recipe: &Recipe, d: Direction, ground_m: f64, footprint_m: f64) -> Column {
    let seed = recipe.seed ^ CAVE;
    let solid = Column {
        seed,
        ground_m,
        crest: 1.0,
        allowed: 0.0,
    };
    // Under the sea a cave is a flood, and a mesh this coarse could not carry
    // one anyway.
    let allowed = band(CAVE_M, footprint_m) * smoothstep(0.0, 40.0, ground_m);
    if allowed <= 0.0 {
        return solid;
    }
    // Caves keep company. Where the rock takes them the bar is low and the
    // ground is riddled; elsewhere nothing reaches it.
    let country = smoothstep(
        0.05,
        0.45,
        fbm(
            seed ^ 2,
            [d[0] * CAVE_REGION, d[1] * CAVE_REGION, d[2] * CAVE_REGION],
            3,
            0.5,
        ),
    );
    if country <= 0.0 {
        return solid;
    }
    Column {
        seed,
        ground_m,
        crest: CAVE_CREST - CAVE_COUNTRY * country,
        allowed,
    }
}

/// How far a point is from the ground, metres, positive inside it.
///
/// The ground is `sample_at`'s height, exactly, and a cave is taken out of it
/// the way a chisel is: the distance to the nearest tunnel wall, whichever of
/// the two is nearer. Subtracting a carving depth instead would make a cave
/// something that has to beat the weight of rock over it, so caves would only
/// ever appear a few metres under the surface. A cave is a place, not a dent.
///
/// Two ridged sums crest along surfaces; where both crest at once the surfaces
/// meet in a line, and a line is a passage. One would give sheets, three would
/// give beads, and this is the cheapest shape that is a tunnel.
///
/// The field is read at the world position, so a cave turns as much going down
/// as going along.
pub fn density_m(column: &Column, d: Direction, height_m: f64) -> f64 {
    let solid_m = column.ground_m - height_m;
    if column.solid() {
        return solid_m;
    }
    // Past the floor of the build band is the bedrock nobody digs through.
    let allowed = column.allowed * smoothstep(CAVE_FLOOR_M, CAVE_FLOOR_M * 0.7, solid_m);
    if allowed <= 0.0 {
        return solid_m;
    }
    let k = (RADIUS_M + height_m) / CAVE_M;
    let p = [d[0] * k, d[1] * k, d[2] * k];
    let ridge = |salt: u64| 1.0 - libm::fabs(fbm(column.seed ^ salt, p, 3, 0.5));
    // Positive inside both crests, and the smaller of the two is the nearer
    // wall.
    let inside = (ridge(0) - column.crest).min(ridge(1) - column.crest);
    // Where a cave is not allowed the wall is pushed away until it is gone,
    // rather than cut off, so the ground has no seam at the edge of the rule.
    let wall_m = -inside * CAVE_WIDTH_M + (1.0 - allowed) * CAVE_WIDTH_M;
    solid_m.min(wall_m)
}
