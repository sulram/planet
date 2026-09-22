//! A plate margin is a slope. It used to be a wall, three different ways.

use topology::{Sector, SurfacePoint};
use worldgen::{Generator, Recipe, parse_seed};

/// Metres the ground may change over one metre of travel. A coast is steep
/// and a headland is steeper, so this is not a slope limit: it is far above
/// anything the relief asks for and far below the walls it stands guard on,
/// which were 762, 528 and 271 m tall.
const CLIFF_M: f64 = 60.0;

/// Places on seed `68881fed2ec32954` that each used to be a vertical wall:
/// the arc against the trench, the regime flipping at a junction, and the
/// crust changing which plate it belonged to.
const WALLS: [(f64, f64); 3] = [(31017.0, 30665.0), (33279.0, 31271.0), (33271.0, 32323.0)];

#[test]
fn a_plate_margin_is_a_slope_and_not_a_wall() {
    let recipe = Recipe::new(parse_seed("68881fed2ec32954").expect("a seed"));
    let generator = Generator::new(recipe).expect("a world");
    let grid = generator.sphere().blocks();
    let at = |u: f64, v: f64| {
        generator
            .sample(grid.direction(SurfacePoint::new(Sector::ALL[0], u, v)))
            .height_m
    };
    for (u, v) in WALLS {
        // Two blocks is one metre, and the wall ran along one axis or the
        // other, so both are worth asking about.
        for (name, drop) in [
            ("u", (at(u + 2.0, v) - at(u - 2.0, v)).abs()),
            ("v", (at(u, v + 2.0) - at(u, v - 2.0)).abs()),
        ] {
            assert!(
                drop < CLIFF_M,
                "a wall {drop:.0} m tall over 2 m along {name} at u {u} v {v}"
            );
        }
    }
}
