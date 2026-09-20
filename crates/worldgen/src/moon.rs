//! The moon of generator v2: an airless ball of craters.
//!
//! Craters come from a cell grid in the space around the unit sphere, several
//! sizes stacked, the way impacts of every size pile up. Each cell may hold
//! one crater: a parabolic bowl with a raised rim. Like the planet, sampling
//! takes a footprint, and craters too small for the mesh that asks fade out.

use crate::noise::{band, fbm, smoothstep};
use crate::{Direction, Material, Recipe, Sample};

const CRATERS: u64 = 0x2_1000;
const ROLL: u64 = 0x2_2000;

/// Radius the sizes below are quoted against, metres.
pub const RADIUS_M: f64 = 8_000.0;

/// Cells per radius for each crater size, big to small.
const SCALES: [f64; 8] = [1.7, 3.0, 5.0, 11.0, 26.0, 60.0, 150.0, 400.0];
/// The first sizes are basins: craters wide enough to survive the coarsest
/// mesh, which is what the planet sees. They read by their relief alone: a
/// deep bowl, a tall rim, and the light across them.
const BASINS: usize = 2;

/// splitmix64 over a cell and a salt, as three numbers in `0..1`.
fn cell_random(seed: u64, cell: [i64; 3], salt: u64) -> [f64; 3] {
    let mut x = seed
        ^ salt
        ^ (cell[0] as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (cell[1] as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
        ^ (cell[2] as u64).wrapping_mul(0x1656_67b1_9e37_79f9);
    let mut next = || {
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    [next(), next(), next()]
}

/// Height of one size of craters at `p` (the direction times the scale), in
/// cell units.
fn craters(seed: u64, p: [f64; 3], salt: u64, basin: bool) -> f64 {
    let scale = libm::sqrt(p[0] * p[0] + p[1] * p[1] + p[2] * p[2]);
    // A crater is at most half a cell wide, so the eight cells around the
    // nearest lattice corner hold every crater that can reach `p`.
    let base = [0, 1, 2].map(|axis| libm::floor(p[axis] - 0.5) as i64);
    let mut height = 0.0;
    for corner in 0..8i64 {
        let cell = [
            base[0] + (corner & 1),
            base[1] + ((corner >> 1) & 1),
            base[2] + (corner >> 2),
        ];
        let [a, b, c] = cell_random(seed, cell, salt);
        let [size, freshness, _] = cell_random(seed, cell, salt ^ 0x5bd1);
        // Basins are rare: a few seas, not a pattern.
        if basin && freshness < 0.55 {
            continue;
        }
        let mut center = [cell[0] as f64 + a, cell[1] as f64 + b, cell[2] as f64 + c];
        if basin {
            // Basin cells are as wide as the moon is deep: most centres would
            // miss the surface. Pull them onto it, so every one is a basin.
            let length =
                libm::sqrt(center[0] * center[0] + center[1] * center[1] + center[2] * center[2]);
            center = center.map(|c| c / length * scale);
        }
        // Basins start wide: the smallest still spans several root vertices.
        let radius = if basin {
            0.24 + 0.18 * size
        } else {
            0.12 + 0.30 * size * size
        };
        let offset = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
        let t = libm::sqrt(offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2])
            / radius;
        if t >= 2.0 {
            continue;
        }
        // Old craters are worn: shallower, softer rims.
        let depth = radius * (0.10 + 0.22 * freshness);
        let bowl = if t < 1.0 { depth * (t * t - 1.0) } else { 0.0 };
        let lip = (t - 1.0) / 0.22;
        let rim = depth
            * if basin { 0.45 } else { 0.30 }
            * libm::exp(-lip * lip)
            * smoothstep(2.0, 1.4, t).max(0.0);
        height += bowl + rim;
    }
    height
}

pub fn sample(recipe: &Recipe, d: Direction, footprint_m: f64) -> Sample {
    let seed = recipe.seed;
    let mut height_m = 120.0 * fbm(seed ^ ROLL, [d[0] * 2.5, d[1] * 2.5, d[2] * 2.5], 5, 0.5);
    for (index, scale) in SCALES.iter().enumerate() {
        let cell_m = RADIUS_M / scale;
        // The smallest crater of this size: a quarter of a cell across, or
        // half for the basins.
        let smallest = if index < BASINS { 0.48 } else { 0.25 };
        let weight = band(cell_m * smallest, footprint_m);
        if weight <= 0.0 {
            break;
        }
        let p = [d[0] * scale, d[1] * scale, d[2] * scale];
        let height = craters(seed ^ CRATERS, p, index as u64, index < BASINS);
        height_m += weight * cell_m * height;
    }
    Sample {
        height_m,
        material: Material::Regolith,
    }
}
