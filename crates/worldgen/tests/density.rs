//! What the volume owes the height, and what a cave may do.
//!
//! The volume layer and the heightfield quadtree draw the same ground at
//! different resolutions, so the one thing that must never drift is where
//! they agree it is.

use worldgen::{Generator, Recipe};

fn directions() -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for i in -4i32..=4 {
        for j in -4i32..=4 {
            for k in -4i32..=4 {
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

#[test]
fn the_ground_is_exactly_where_the_height_says() {
    // Not "close to": the handover between a volume chunk and a heightfield
    // patch has nothing to reconcile only if this is exact.
    let generator = Generator::new(Recipe::new(7)).unwrap();
    for d in directions() {
        let ground_m = generator.sample(d).height_m;
        let density = generator.density_m(d, ground_m, 0.0);
        // A cave may carve the surface itself; away from one it is zero.
        assert!(
            density <= 0.0,
            "{d:?} floats {density} m over its own ground"
        );
        if density == 0.0 {
            continue;
        }
        assert!(
            generator.density_m(d, ground_m + 1.0, 0.0) < 0.0,
            "{d:?} is solid above its own ground"
        );
    }
}

#[test]
fn air_is_above_and_ground_is_below() {
    let generator = Generator::new(Recipe::new(7)).unwrap();
    for d in directions() {
        let ground_m = generator.sample(d).height_m;
        assert!(generator.density_m(d, ground_m + 50.0, 0.0) < 0.0);
        // Under the floor a cave may reach, the ground is ground.
        assert!(generator.density_m(d, ground_m - 400.0, 0.0) > 0.0);
    }
}

#[test]
fn a_world_has_caves_in_it() {
    // Without this the volume layer buys nothing a height did not already
    // give: a tunnel you dig, and never one you find.
    let generator = Generator::new(Recipe::new(7)).unwrap();
    let mut found = 0;
    for d in directions() {
        let ground_m = generator.sample(d).height_m;
        if ground_m <= 60.0 {
            continue;
        }
        for step in 1..=110 {
            if generator.density_m(d, ground_m - f64::from(step), 0.0) < 0.0 {
                found += 1;
                break;
            }
        }
    }
    assert!(
        found > 0,
        "no cave anywhere under {} directions",
        directions().len()
    );
}

#[test]
fn nothing_is_hollow_under_the_sea_or_under_the_bedrock() {
    let generator = Generator::new(Recipe::new(7)).unwrap();
    for d in directions() {
        let ground_m = generator.sample(d).height_m;
        if ground_m < 0.0 {
            // A cave under the sea floor is a flood.
            for step in 1..=200 {
                let at = ground_m - f64::from(step);
                assert!(
                    generator.density_m(d, at, 0.0) > 0.0,
                    "the sea floor is hollow at {d:?}"
                );
            }
        }
        // Past the floor of the build band is bedrock, whatever the ground.
        for step in 140..=300 {
            let at = ground_m - f64::from(step);
            assert!(
                generator.density_m(d, at, 0.0) > 0.0,
                "bedrock is hollow at {d:?}"
            );
        }
    }
}

#[test]
fn a_frozen_world_is_solid_all_the_way_down() {
    // v1 and v2 never had a volume, so they never had a cave, and giving them
    // one now would change ground under chunks saved against them.
    for version in [1, 2] {
        let generator = Generator::new(Recipe {
            generator_version: version,
            ..Recipe::new(7)
        })
        .unwrap();
        for d in directions() {
            let ground_m = generator.sample(d).height_m;
            for step in 1..=200 {
                let at = ground_m - f64::from(step);
                assert_eq!(generator.density_m(d, at, 0.0), ground_m - at);
            }
        }
    }
}

#[test]
fn a_mesh_too_coarse_for_a_cave_is_not_shown_one() {
    // The same rule every octave follows: what a mesh cannot carry fades out
    // instead of aliasing.
    let generator = Generator::new(Recipe::new(7)).unwrap();
    for d in directions() {
        let ground_m = generator.sample(d).height_m;
        for step in 1..=110 {
            let at = ground_m - f64::from(step);
            let coarse = generator.density_m(d, at, 600.0);
            let plain = generator.sample_at(d, 600.0).height_m - at;
            assert_eq!(coarse, plain, "a cave survived a 600 m footprint at {d:?}");
        }
    }
}
