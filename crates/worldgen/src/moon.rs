//! The moon of generator v2: an airless ball of craters.
//!
//! Craters come from a cell grid in the space around the unit sphere, several
//! sizes stacked, the way impacts of every size pile up. Each cell may hold
//! one crater: a parabolic bowl with a raised rim. Like the planet, sampling
//! takes a footprint, and craters too small for the mesh that asks fade out.
//!
//! Sampling runs on the frame's thread in the browser, where a cell costs
//! several times what it does natively: the cost of a sample is a design
//! constraint here, and it is measured in WASM.

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

/// The widest bowl of any size, in cells. The rim reaches twice as far.
const WIDEST: f64 = 0.42;
/// A basin's cell centre is pulled onto the surface from at most this far, in
/// cells: cells deeper inside the moon or further out hold no basin.
const BASIN_SHELL: f64 = 0.65;

/// One crater, in the cell units of its size.
#[derive(Clone, Copy, Debug)]
struct Crater {
    center: [f64; 3],
    radius: f64,
    depth: f64,
    /// Height of the rim, as a share of the depth.
    rim: f64,
}

impl Crater {
    /// A parabolic bowl, and a rim that fades out at twice the radius.
    fn height(&self, p: [f64; 3]) -> f64 {
        let offset = [0, 1, 2].map(|axis| p[axis] - self.center[axis]);
        let squared = offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2];
        let reach = 2.0 * self.radius;
        if squared >= reach * reach {
            return 0.0;
        }
        let t = libm::sqrt(squared) / self.radius;
        let bowl = if t < 1.0 {
            self.depth * (t * t - 1.0)
        } else {
            0.0
        };
        let lip = (t - 1.0) / 0.22;
        let rim = self.depth * self.rim * libm::exp(-lip * lip) * smoothstep(2.0, 1.4, t).max(0.0);
        bowl + rim
    }
}

/// Old craters are worn: shallower, softer rims.
fn depth(radius: f64, freshness: f64) -> f64 {
    radius * (0.10 + 0.22 * freshness)
}

/// The basins of one moon, per basin size: few enough to list once, when the
/// generator is made, instead of searching cells at every sample. Their cells
/// are as wide as the moon is deep, so most centres would miss the surface:
/// those within `BASIN_SHELL` of it are pulled onto it and are basins, and the
/// rest hold nothing.
#[derive(Clone, Debug)]
pub struct Basins([Vec<Crater>; BASINS]);

impl Basins {
    pub fn new(recipe: &Recipe) -> Basins {
        let seed = recipe.seed ^ CRATERS;
        Basins(core::array::from_fn(|index| {
            let (scale, salt) = (SCALES[index], index as u64);
            let last = libm::floor(scale + BASIN_SHELL) as i64;
            let mut basins = Vec::new();
            for k in -last - 1..=last {
                for j in -last - 1..=last {
                    for i in -last - 1..=last {
                        let cell = [i, j, k];
                        let [a, b, c] = cell_random(seed, cell, salt);
                        let center = [i as f64 + a, j as f64 + b, k as f64 + c];
                        let length = libm::sqrt(
                            center[0] * center[0] + center[1] * center[1] + center[2] * center[2],
                        );
                        let [size, freshness, _] = cell_random(seed, cell, salt ^ 0x5bd1);
                        // Basins are rare: a few seas, not a pattern.
                        if libm::fabs(length - scale) > BASIN_SHELL || freshness < 0.55 {
                            continue;
                        }
                        // They start wide: the smallest still spans several
                        // root vertices.
                        let radius = 0.24 + (WIDEST - 0.24) * size;
                        basins.push(Crater {
                            center: center.map(|c| c / length * scale),
                            radius,
                            depth: depth(radius, freshness),
                            rim: 0.45,
                        });
                    }
                }
            }
            basins
        }))
    }
}

/// Height of one size of craters at `p` (the direction times the scale), in
/// cell units.
fn craters(seed: u64, p: [f64; 3], salt: u64) -> f64 {
    // Every cell whose crater can reach `p` is visited, or the crater would
    // end in a cliff where the search stops seeing it. A rim reaches
    // `2 * WIDEST` from its centre: under one cell, so the cells around the
    // one that holds `p`.
    let reach = 2.0 * WIDEST;
    let home = [0, 1, 2].map(|axis| libm::floor(p[axis]));
    // How far `p` is from each neighbour cell along an axis: below, same, above.
    let gap = [0, 1, 2].map(|axis| {
        let inside = p[axis] - home[axis];
        [inside * inside, 0.0, (1.0 - inside) * (1.0 - inside)]
    });
    let mut height = 0.0;
    for k in 0..3 {
        for j in 0..3 {
            for i in 0..3 {
                // Nothing in a cell this far reaches `p`: skip it before
                // paying for the hash.
                if gap[0][i] + gap[1][j] + gap[2][k] >= reach * reach {
                    continue;
                }
                let cell = [
                    home[0] as i64 + i as i64 - 1,
                    home[1] as i64 + j as i64 - 1,
                    home[2] as i64 + k as i64 - 1,
                ];
                let [a, b, c] = cell_random(seed, cell, salt);
                let center = [cell[0] as f64 + a, cell[1] as f64 + b, cell[2] as f64 + c];
                let offset = [0, 1, 2].map(|axis| p[axis] - center[axis]);
                if offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]
                    >= reach * reach
                {
                    continue;
                }
                let [size, freshness, _] = cell_random(seed, cell, salt ^ 0x5bd1);
                let radius = 0.12 + (WIDEST - 0.12) * size * size;
                let crater = Crater {
                    center,
                    radius,
                    depth: depth(radius, freshness),
                    rim: 0.30,
                };
                height += crater.height(p);
            }
        }
    }
    height
}

pub fn sample(recipe: &Recipe, basins: &Basins, d: Direction, footprint_m: f64) -> Sample {
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
        let height = match basins.0.get(index) {
            Some(basins) => basins.iter().map(|basin| basin.height(p)).sum(),
            None => craters(seed ^ CRATERS, p, index as u64),
        };
        height_m += weight * cell_m * height;
    }
    Sample {
        height_m,
        material: Material::Regolith,
    }
}
