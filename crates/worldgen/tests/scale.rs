//! A world's size is a print scale, not a different world.
//!
//! The generator versions are written in reference metres and frozen there.
//! `sector_bits` decides how large that shape is printed, so the same seed
//! keeps its coastline and its mountains at any size, and the reference size
//! is untouched down to the bit.

use topology::{MAX_BITS, MIN_BITS, QuadSphere};
use worldgen::{Generator, Recipe};

fn generator(bits: u32) -> Generator {
    let mut recipe = Recipe::new(1);
    recipe.sector_bits = bits;
    Generator::new(recipe).expect("a generator")
}

/// Directions spread over the sphere, enough to meet sea and land.
fn directions() -> impl Iterator<Item = [f64; 3]> {
    (0..2000).map(|i| {
        let a = f64::from(i) * 1e-3;
        let d = [a.cos(), (a * 0.7).sin(), (a * 1.3).sin()];
        let n = libm::sqrt(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
        [d[0] / n, d[1] / n, d[2] / n]
    })
}

/// Ground range over the sphere, in metres.
fn relief_m(g: &Generator) -> f64 {
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for d in directions() {
        let h = g.sample_at(d, 1.0 * g.scale()).height_m;
        lo = lo.min(h);
        hi = hi.max(h);
    }
    hi - lo
}

/// The reference body pays nothing for the others existing: its scale is
/// exactly one, and multiplying by one changes no bit.
#[test]
fn the_reference_size_is_untouched() {
    let g = generator(MAX_BITS);
    assert_eq!(g.scale(), 1.0);
}

/// Relief follows the radius, so no body is a billiard ball and none is a
/// bed of spikes taller than itself.
#[test]
fn relief_keeps_its_share_of_the_radius() {
    let reference = {
        let g = generator(MAX_BITS);
        relief_m(&g) / g.sphere().radius_m()
    };
    for bits in MIN_BITS..=MAX_BITS {
        let g = generator(bits);
        let share = relief_m(&g) / g.sphere().radius_m();
        assert!(
            (share - reference).abs() < reference * 1e-9,
            "bits {bits}: relief is {share} of the radius, reference is {reference}"
        );
    }
}

/// The scale is an exact power of two, so a small world is not a rounded one.
#[test]
fn the_scale_is_exact() {
    for bits in MIN_BITS..=MAX_BITS {
        let g = generator(bits);
        let side = f64::from(QuadSphere::new(bits).unwrap().blocks().side());
        assert_eq!(g.scale() * f64::from(1u32 << MAX_BITS), side, "bits {bits}");
    }
}

/// A size no quad sphere can have is refused, not clamped.
#[test]
fn an_impossible_size_is_refused() {
    for bits in [0, 1, MIN_BITS - 1, MAX_BITS + 1, 32] {
        let mut recipe = Recipe::new(1);
        recipe.sector_bits = bits;
        assert!(Generator::new(recipe).is_err(), "bits {bits}");
    }
}
