//! Generator version 2. Frozen once a release ships it.
//!
//! What changed from v1, and why:
//! - **Eroded mountains.** v1 squared a ridged sum, which made needles. Here
//!   each octave is damped by the slope accumulated so far (after Quilez), so
//!   detail gathers on gentle ground and steep faces stay clean, the way
//!   erosion leaves them. Crests are rounded, not creased.
//! - **A real sea floor.** Shelf, slope, abyssal plain and seamounts, because
//!   the sea is penetrable now.
//! - **Filtered sampling.** `footprint_m` fades octaves finer than the mesh
//!   that asks, so coarse patches do not alias and LOD changes do not pop.
//!   Collision and the save format use `footprint_m = 0`: the full terrain.

use crate::noise::{band, fbm, simplex_d, smoothstep};
use crate::{Direction, Material, Recipe, Sample};

const CONTINENT: u64 = 0x1_1000;
const WARP: u64 = 0x1_2000;
const RANGE: u64 = 0x1_3000;
const MOUNTAIN: u64 = 0x1_4000;
const HILL: u64 = 0x1_5000;
const DETAIL: u64 = 0x1_6000;
const MOISTURE: u64 = 0x1_7000;
const SEABED: u64 = 0x1_8000;

/// Radius the wavelengths below are quoted against, metres. A constant of the
/// generator, not of the topology: changing it would change the terrain.
const RADIUS_M: f64 = 20_860.0;

/// How hard accumulated slope damps the finer octaves.
const EROSION: f64 = 0.06;
/// Amplitude kept from one octave to the next.
const GAIN: f64 = 0.58;

/// A fractal sum whose octaves are damped by the slope gathered so far (after
/// Quilez), and faded by `band` below the sampling footprint. About `-1..=1`.
///
/// Steep ground takes less of each finer octave, gentle ground takes all of
/// it: massifs stay broad and clean, detail settles in valleys and on
/// shoulders, the way erosion leaves a range. `sharpness` in `0..=1` folds the
/// finer octaves into rounded crests, so slopes carry ridgelines and gullies
/// without the first octaves ever becoming blades.
///
/// `frequency` is in cycles per planet radius; `footprint` in the same unit
/// sphere space (`footprint_m / RADIUS_M`).
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
        // The first two octaves shape the massif and stay smooth.
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

/// `1 + GAIN + GAIN^2 ...` over all octaves, faded or not: a filtered sum is the
/// same terrain with its fine octaves at their mean, never a rescaled one.
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

pub fn sample(recipe: &Recipe, d: Direction, footprint_m: f64) -> Sample {
    let seed = recipe.seed;
    let params = &recipe.params;
    let footprint = footprint_m / RADIUS_M;
    let scale = params.continent_scale;
    let at = |k: f64| [d[0] * k, d[1] * k, d[2] * k];

    // Continents: as v1, which read well from orbit.
    let warp = fbm(seed ^ WARP, at(scale * 2.0), 3, 0.5) * 0.35;
    let c = at(scale);
    let continent = fbm(
        seed ^ CONTINENT,
        [c[0] + warp, c[1] - warp, c[2] + warp],
        6,
        0.5,
    );
    let land = continent - (params.sea_share - 0.5) * 0.9;

    let height_m = if land < 0.0 {
        let sea = -land;
        // Shelf (shallow, wide), slope, then the plain.
        let shelf = smoothstep(0.0, 0.06, sea) * 0.06;
        let deep = smoothstep(0.05, 0.40, sea) * 0.94;
        let relief = filtered(seed ^ SEABED, d, scale * 10.0, 6, footprint);
        let seamounts = eroded(seed ^ SEABED ^ 1, d, scale * 5.0, 6, 0.5, footprint).max(0.0);
        let ripples = filtered(seed ^ DETAIL, d, 900.0, 5, footprint);
        -params.ocean_depth_m * (shelf + deep) * (1.0 - 0.35 * seamounts * deep)
            + deep * 40.0 * relief
            + smoothstep(0.0, 0.02, sea) * 0.6 * ripples
            - 0.4
    } else {
        // Ease in from the coast so beaches are wide and flat.
        let inland = smoothstep(0.0, 0.22, land);
        let coast = smoothstep(0.0, 0.05, land);
        // Where mountain ranges are allowed to rise.
        let ranges = smoothstep(
            0.05,
            0.55,
            fbm(seed ^ RANGE, at(scale * 2.5), 4, 0.5) + 0.15,
        );
        let mountains = 0.5 + 0.5 * eroded(seed ^ MOUNTAIN, d, scale * 5.0, 12, 0.9, footprint);
        let hills = filtered(seed ^ HILL, d, scale * 30.0, 8, footprint);
        let detail = filtered(seed ^ DETAIL, d, 1800.0, 5, footprint);
        params.relief_m * inland * (0.03 + 0.07 * land + 0.90 * ranges * mountains)
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
