//! Seeded 3D simplex noise and the fractal sums built on it.
//!
//! The lattice hash is integer only, so the seed changes the whole field and
//! no permutation table is needed. The algorithm is Gustavson's public domain
//! simplex noise; the code is ours.

const F3: f64 = 1.0 / 3.0;
const G3: f64 = 1.0 / 6.0;

/// The 12 edge midpoints of a cube.
const GRADIENTS: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// Mixes a lattice corner with the seed (splitmix64 finalizer).
fn hash(seed: u64, i: i64, j: i64, k: i64) -> u64 {
    let mut x = seed
        ^ (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (j as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
        ^ (k as u64).wrapping_mul(0x1656_67b1_9e37_79f9);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Contribution of one simplex corner.
fn corner(seed: u64, i: i64, j: i64, k: i64, x: f64, y: f64, z: f64) -> f64 {
    let t = 0.6 - x * x - y * y - z * z;
    if t <= 0.0 {
        return 0.0;
    }
    let g = GRADIENTS[(hash(seed, i, j, k) % 12) as usize];
    let t2 = t * t;
    t2 * t2 * (g[0] * x + g[1] * y + g[2] * z)
}

/// Simplex noise, about `-1..=1`.
pub fn simplex(seed: u64, p: [f64; 3]) -> f64 {
    let [x, y, z] = p;
    let s = (x + y + z) * F3;
    let (fi, fj, fk) = (libm::floor(x + s), libm::floor(y + s), libm::floor(z + s));
    let t = (fi + fj + fk) * G3;
    let (x0, y0, z0) = (x - (fi - t), y - (fj - t), z - (fk - t));
    let (i, j, k) = (fi as i64, fj as i64, fk as i64);

    // Which of the six tetrahedra of the cell holds the point.
    let (o1, o2): ([i64; 3], [i64; 3]) = if x0 >= y0 {
        if y0 >= z0 {
            ([1, 0, 0], [1, 1, 0])
        } else if x0 >= z0 {
            ([1, 0, 0], [1, 0, 1])
        } else {
            ([0, 0, 1], [1, 0, 1])
        }
    } else if y0 < z0 {
        ([0, 0, 1], [0, 1, 1])
    } else if x0 < z0 {
        ([0, 1, 0], [0, 1, 1])
    } else {
        ([0, 1, 0], [1, 1, 0])
    };

    let at = |o: [i64; 3], shift: f64| {
        corner(
            seed,
            i + o[0],
            j + o[1],
            k + o[2],
            x0 - o[0] as f64 + shift,
            y0 - o[1] as f64 + shift,
            z0 - o[2] as f64 + shift,
        )
    };
    32.0 * (at([0, 0, 0], 0.0) + at(o1, G3) + at(o2, 2.0 * G3) + at([1, 1, 1], 3.0 * G3))
}

/// Fractal sum: each octave doubles the frequency and scales by `gain`.
/// Normalised to about `-1..=1`. Octaves get their own seed so they do not
/// line up at the origin.
pub fn fbm(seed: u64, p: [f64; 3], octaves: u32, gain: f64) -> f64 {
    let (mut sum, mut norm, mut amplitude, mut frequency) = (0.0, 0.0, 1.0, 1.0);
    for octave in 0..octaves {
        let q = [p[0] * frequency, p[1] * frequency, p[2] * frequency];
        sum += amplitude * simplex(seed.wrapping_add(u64::from(octave)), q);
        norm += amplitude;
        amplitude *= gain;
        frequency *= 2.0;
    }
    sum / norm
}

/// Ridged fractal sum, `0..=1`: sharp crests where the noise crosses zero.
/// Each octave is weighted by the one before, so ridges carry the detail and
/// valleys stay smooth.
pub fn ridged(seed: u64, p: [f64; 3], octaves: u32, gain: f64) -> f64 {
    let (mut sum, mut norm, mut amplitude, mut frequency, mut weight) = (0.0, 0.0, 1.0, 1.0, 1.0);
    for octave in 0..octaves {
        let q = [p[0] * frequency, p[1] * frequency, p[2] * frequency];
        let crest = 1.0 - libm::fabs(simplex(seed.wrapping_add(u64::from(octave)), q));
        let crest = crest * crest * weight;
        sum += amplitude * crest;
        norm += amplitude;
        weight = crest.clamp(0.0, 1.0);
        amplitude *= gain;
        frequency *= 2.0;
    }
    sum / norm
}

pub fn smoothstep(lo: f64, hi: f64, x: f64) -> f64 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
