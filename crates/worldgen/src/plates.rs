//! Tectonic plates: where the land is, and where it is allowed to rise.
//!
//! Generator v2 drew mountains from a blob of noise, so ranges came out as
//! patches. On a real body a range is the edge of a plate, which is why they
//! run in arcs thousands of kilometres long and why a trench always faces one.
//! Here a few plates cover the sphere, each with a rotation of its own, and
//! every sample reads the boundary between the two nearest: convergence
//! raises a cordillera or digs a trench, divergence opens a ridge or a rift.
//!
//! The plates are the same for every sample, so they are laid out once, in
//! `Generator::new`, and never searched for per sample (CLAUDE.md,
//! Performance). What a sample costs is `COUNT` dot products and two noise
//! calls, well under one octave of the relief that sits on top.

use crate::noise::{fbm, simplex_d, smoothstep};
use crate::{Direction, Recipe};

const SITE: u64 = 0x3_1000;
const WARP: u64 = 0x3_2000;
const COAST: u64 = 0x3_3000;
const BELT: u64 = 0x3_4000;

/// How many plates cover the sphere. Earth has seven majors and about a dozen
/// minors; this many gives continents that read as continents at our radius.
const COUNT: usize = 18;
/// Rounds of repulsion that turn random sites into a spread of plates. Sites
/// still clump a little, which is what makes plates differ in size.
const RELAX: u32 = 32;
/// Share of plates that carry continental crust: the rest are ocean floor.
const CONTINENTAL: f64 = 0.42;

/// Angle, in radians, over which two plates blend into each other. Wider than
/// a boundary feature: crust thins toward a margin before it ends.
const MARGIN: f64 = 0.16;
/// Angle a boundary feature reaches inland. A cordillera is narrow next to
/// the plate that carries it.
const REACH: f64 = 0.075;

/// Where the land is and where it rises: what every source of shape gives the
/// body of the generator.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shape {
    /// Signed continentality. Negative is sea, and the sea deepens with it;
    /// positive is land, and `1` is as far inland as a body gets. This one is
    /// read at the mesh's own resolution: it decides where the coast is, and
    /// the terms it feeds all saturate, so a sharp coast is a narrow beach
    /// and never a wall.
    pub land: f64,
    /// How high the ground sits before its own relief, `0..=1`. Broad on
    /// purpose: a slope of this is a slope of the whole continent.
    pub lift: f64,
    /// Where mountain belts may rise, `0..=1`. Broad for the same reason.
    pub ranges: f64,
}

#[derive(Clone, Copy, Debug)]
struct Plate {
    /// Unit vector through the middle of the plate.
    site: [f64; 3],
    /// Euler pole times angular speed: `spin x p` is the plate's velocity.
    spin: [f64; 3],
    /// Continental crust floats high; oceanic crust sits low and subducts.
    continental: bool,
}

/// The plates of one world.
#[derive(Clone, Debug)]
pub struct Plates([Plate; COUNT]);

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    let length = libm::sqrt(dot(a, a)).max(1e-12);
    [a[0] / length, a[1] / length, a[2] / length]
}

/// splitmix64 over a plate index and a salt, as one number in `0..1`.
fn plate_random(seed: u64, index: usize, salt: u64) -> f64 {
    let mut x = seed ^ salt ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((x ^ (x >> 31)) >> 11) as f64 / (1u64 << 53) as f64
}

/// A unit vector spread evenly over the sphere, from two numbers in `0..1`.
fn on_sphere(a: f64, b: f64) -> [f64; 3] {
    let z = 2.0 * a - 1.0;
    let r = libm::sqrt((1.0 - z * z).max(0.0));
    let angle = b * core::f64::consts::TAU;
    [r * libm::cos(angle), r * libm::sin(angle), z]
}

impl Plates {
    pub fn new(recipe: &Recipe) -> Plates {
        let seed = recipe.seed ^ SITE;
        let mut sites: [[f64; 3]; COUNT] = core::array::from_fn(|index| {
            on_sphere(
                plate_random(seed, index, 0),
                plate_random(seed, index, 0x11),
            )
        });
        // Push the sites apart a few rounds: random alone leaves plates that
        // are slivers, and a sliver reads as a scar, not as a plate.
        for _ in 0..RELAX {
            let before = sites;
            for (index, site) in sites.iter_mut().enumerate() {
                let mut push = [0.0f64; 3];
                for (other, away) in before.iter().enumerate() {
                    if other == index {
                        continue;
                    }
                    // Closer sites push harder, and only over the near half.
                    let gap = (1.0 - dot(*site, *away)).max(0.02);
                    let weight = 0.05 / (gap * gap);
                    for axis in 0..3 {
                        push[axis] += weight * (site[axis] - away[axis]);
                    }
                }
                *site = normalize([site[0] + push[0], site[1] + push[1], site[2] + push[2]]);
            }
        }
        Plates(core::array::from_fn(|index| Plate {
            site: sites[index],
            spin: {
                let pole = on_sphere(
                    plate_random(seed, index, 0x21),
                    plate_random(seed, index, 0x22),
                );
                let speed = 0.4 + 0.6 * plate_random(seed, index, 0x23);
                [pole[0] * speed, pole[1] * speed, pole[2] * speed]
            },
            continental: plate_random(seed, index, 0x31) < CONTINENTAL,
        }))
    }

    /// The two nearest plates at a direction, nearest first, and the angle
    /// from the boundary between them.
    fn boundary(&self, d: Direction) -> (&Plate, &Plate, f64) {
        let (mut best, mut next) = (0usize, 1usize);
        let (mut best_dot, mut next_dot) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (index, plate) in self.0.iter().enumerate() {
            let value = dot(d, plate.site);
            if value > best_dot {
                (next, next_dot) = (best, best_dot);
                (best, best_dot) = (index, value);
            } else if value > next_dot {
                (next, next_dot) = (index, value);
            }
        }
        let (a, b) = (&self.0[best], &self.0[next]);
        // The boundary is the plane that bisects the two sites. The angle to
        // it is the gap in the dot products over how far apart the sites are.
        let split = [0, 1, 2].map(|axis| a.site[axis] - b.site[axis]);
        let edge = (best_dot - next_dot) / libm::sqrt(dot(split, split)).max(1e-9);
        (a, b, edge)
    }

    /// The shape of the crust under a direction.
    pub fn shape(&self, seed: u64, d: Direction, sea_share: f64) -> Shape {
        // Plate edges are not great circles. One noise call gives a whole warp
        // vector through its analytic gradient, so the boundaries wander for
        // the price of a single octave.
        let (_, coarse) = simplex_d(seed ^ WARP, [d[0] * 1.9, d[1] * 1.9, d[2] * 1.9]);
        let (_, fine) = simplex_d(seed ^ WARP ^ 1, [d[0] * 5.1, d[1] * 5.1, d[2] * 5.1]);
        let warped =
            normalize([0, 1, 2].map(|axis| d[axis] + 0.055 * coarse[axis] + 0.018 * fine[axis]));

        let (near, far, edge) = self.boundary(warped);

        // Crust: continental floats, oceanic sits low, and the two blend over
        // the margin so a coast is a slope and not a wall.
        let level = |plate: &Plate| if plate.continental { 0.46 } else { -0.52 };
        let across = 0.5 + 0.5 * smoothstep(0.0, MARGIN, edge);
        let mut land = level(far) + (level(near) - level(far)) * across;
        let mut ranges = 0.0;

        // How the two plates move against each other, at this point, along the
        // boundary normal. Positive closes the gap.
        let normal = normalize([0, 1, 2].map(|axis| near.site[axis] - far.site[axis]));
        let motion = {
            let here = cross(far.spin, warped);
            let there = cross(near.spin, warped);
            dot([0, 1, 2].map(|axis| here[axis] - there[axis]), normal)
        };
        let strength = (libm::fabs(motion) * 1.5).min(1.0);
        let close = 1.0 - smoothstep(0.0, REACH, edge);

        if motion > 0.0 {
            // Convergent. Two continents pile up into one belt; where ocean
            // meets land the ocean dives, leaving a trench on its side and an
            // arc of volcanoes on the other.
            let force = close * strength;
            match (near.continental, far.continental) {
                (true, true) => {
                    land += 0.30 * force;
                    ranges += force;
                }
                (true, false) => {
                    land += 0.16 * force;
                    ranges += 0.95 * force;
                }
                (false, true) => {
                    land -= 0.85 * force;
                    ranges += 0.25 * force;
                }
                (false, false) => {
                    // Ocean meets ocean: a trench and an island arc beside it.
                    land -= 0.55 * force;
                    ranges += 0.55 * force;
                }
            }
        } else {
            // Divergent, or sliding past. New crust rises along an oceanic
            // ridge; a continent pulled apart drops into a rift valley.
            let force = close * strength;
            if near.continental && far.continental {
                land -= 0.34 * force;
                ranges += 0.30 * force;
            } else {
                land += 0.26 * force;
                ranges += 0.22 * force;
            }
        }

        // Coastlines are fractal, so the crust carries noise at every scale
        // the eye reads from orbit down to a bay.
        land += 0.34 * fbm(seed ^ COAST, [d[0] * 3.1, d[1] * 3.1, d[2] * 3.1], 6, 0.5);
        // A belt is not uniform along its length: passes and massifs.
        ranges *= 0.55 + 0.45 * fbm(seed ^ BELT, [d[0] * 7.0, d[1] * 7.0, d[2] * 7.0], 3, 0.5);

        let land = land - (sea_share - 0.5) * 0.9;
        Shape {
            land,
            // Plate crust is already broad: its own height is the lift.
            lift: smoothstep(0.0, 0.9, land),
            ranges: ranges.clamp(0.0, 1.0),
        }
    }
}
