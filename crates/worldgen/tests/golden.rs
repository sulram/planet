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
fn fingerprint(generator_version: u32, seed: u64) -> u64 {
    let recipe = Recipe {
        generator_version,
        ..Recipe::new(seed)
    };
    let generator = Generator::new(recipe).unwrap();
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
    let got: Vec<u64> = SEEDS.map(|seed| fingerprint(1, seed)).to_vec();
    assert_eq!(got, GOLDEN_V1, "generator v1 changed its output");
}

const SEEDS: [u64; 4] = [0, 1, 0x0000_0000_dead_beef, u64::MAX];

#[test]
fn v3_is_frozen() {
    let got: Vec<u64> = SEEDS.map(|seed| fingerprint(3, seed)).to_vec();
    assert_eq!(got, GOLDEN_V3, "generator v3 changed its output");
}

const GOLDEN_V3: [u64; 4] = [
    0xed12_aeda_06d7_8008,
    0x0031_791e_1f9b_b2c9,
    0xb8e2_df13_eede_7796,
    0x644f_108f_3349_caf3,
];

#[test]
fn v2_is_frozen() {
    let got: Vec<u64> = SEEDS.map(|seed| fingerprint(2, seed)).to_vec();
    assert_eq!(got, GOLDEN_V2, "generator v2 changed its output");
}

const GOLDEN_V2: [u64; 4] = [
    0xa8e6_9797_1a3e_5005,
    0xc797_b223_bb9d_74ee,
    0xd314_7448_0b4f_f2d4,
    0xcf7c_b5e9_b72f_cd56,
];

const GOLDEN_V1: [u64; 4] = [
    0xfae5_97aa_cb18_73bf,
    0x3496_7a03_4c35_779f,
    0x3dc4_fa37_b1fc_77a8,
    0x7d90_535e_723d_d9ee,
];

/// FNV-1a over the moon of generator v2.
#[test]
fn v2_moon_is_frozen() {
    let generator = Generator::new(Recipe::new(1)).unwrap();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for d in directions() {
        for byte in generator
            .moon_sample_at(d, 0.0)
            .height_m
            .to_bits()
            .to_le_bytes()
        {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    assert_eq!(hash, GOLDEN_V2_MOON, "the moon of generator v2 changed");
}

const GOLDEN_V2_MOON: u64 = 0x4efc_b822_ba9b_f9a8;

/// A crater the search stops seeing ends in a cliff. Along great circles in
/// half metre steps the moon's ground never jumps: the steepest wall is a
/// slope, not a step.
#[test]
fn moon_has_no_cliffs() {
    let generator = Generator::new(Recipe::new(1)).unwrap();
    let step = 0.5 / worldgen::MOON_RADIUS_M;
    for circle in 0..3 {
        let tilt = f64::from(circle) * 0.9;
        let mut last: Option<f64> = None;
        for i in 0..(core::f64::consts::TAU / step) as u32 {
            let a = f64::from(i) * step;
            let d = [a.cos() * tilt.cos(), a.sin(), a.cos() * tilt.sin()];
            let height = generator.moon_sample_at(d, 0.0).height_m;
            if let Some(last) = last {
                let jump = (height - last).abs();
                assert!(jump < 2.0, "the moon jumps {jump:.1} m at {d:?}");
            }
            last = Some(height);
        }
    }
}

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
    // v2 adds relief to the sea floor, so it dips a little under the nominal depth.
    assert!((-600.0..-100.0).contains(&bottom));
}
