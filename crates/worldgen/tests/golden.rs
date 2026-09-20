//! Golden hashes per generator version (CLAUDE.md, Tests).
//!
//! The same numbers must come out on every platform. If this test fails you
//! changed a released generator: revert, and put the new terrain in a new
//! version module instead.

use worldgen::{Generator, Recipe};

/// Directions from small integers only, so the inputs are exact everywhere.
fn directions() -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for i in -6i32..=6 {
        for j in -6i32..=6 {
            for k in -6i32..=6 {
                if (i, j, k) != (0, 0, 0) {
                    let v = [f64::from(i), f64::from(j), f64::from(k)];
                    let len = libm::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
                    out.push([v[0] / len, v[1] / len, v[2] / len]);
                }
            }
        }
    }
    out
}

/// FNV-1a over the exact bits of every sample.
fn fingerprint(seed: u64) -> u64 {
    let generator = Generator::new(Recipe::new(seed)).unwrap();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for d in directions() {
        let sample = generator.sample(d);
        let bytes = sample.height_m.to_bits().to_le_bytes();
        for byte in bytes.into_iter().chain([sample.material as u8]) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

#[test]
fn v1_is_frozen() {
    let got: Vec<u64> = [0, 1, 0x0000_0000_dead_beef, u64::MAX]
        .map(fingerprint)
        .to_vec();
    assert_eq!(got, GOLDEN_V1, "generator v1 changed its output");
}

const GOLDEN_V1: [u64; 4] = [
    0xfae5_97aa_cb18_73bf,
    0x3496_7a03_4c35_779f,
    0x3dc4_fa37_b1fc_77a8,
    0x7d90_535e_723d_d9ee,
];

#[test]
fn terrain_is_plausible() {
    let generator = Generator::new(Recipe::new(7)).unwrap();
    let samples: Vec<_> = directions()
        .into_iter()
        .map(|d| generator.sample(d))
        .collect();
    let sea = samples.iter().filter(|s| s.height_m < 0.0).count() as f64 / samples.len() as f64;
    let top = samples.iter().map(|s| s.height_m).fold(f64::MIN, f64::max);
    let bottom = samples.iter().map(|s| s.height_m).fold(f64::MAX, f64::min);
    println!("sea share {sea:.2}, heights {bottom:.0} m to {top:.0} m");
    assert!((0.3..0.8).contains(&sea));
    assert!(top > 200.0 && top < 1600.0);
    assert!((-500.0..-100.0).contains(&bottom));
}
