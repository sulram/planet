//! Temporary: which term makes the wall?
use topology::{Sector, SurfacePoint};
use worldgen::{Field, Generator, Recipe, Source};

const NAMES: [&str; 6] = ["base", "lift", "ranges*mtn", "hills", "coast", "land"];

#[test]
fn where_is_the_wall() {
    let bytes = std::fs::read("../../assets/fields/earth.field").unwrap();
    let field = Field::parse(bytes).unwrap();
    let mut recipe = Recipe::new(1);
    recipe.params.source = Source::Field(field.id());
    let generator = Generator::with_field(recipe, field).unwrap();
    let grid = generator.sphere().blocks();
    let side = f64::from(grid.side());
    let radius_m = generator.sphere().radius_m();
    let span_m = 20_000.0;
    let n = 1000;
    let step_m = span_m / n as f64;
    let du = span_m / radius_m * side * 2.0 / core::f64::consts::PI;
    let at = |i: usize| {
        let u = 0.62 * side + du * (i as f64 / n as f64 - 0.5);
        grid.direction(SurfacePoint::new(Sector::ALL[0], u, 0.083 * side))
    };
    let mut worst = (0.0f64, 0usize);
    let mut land_grades: Vec<f64> = Vec::new();
    for i in 0..n - 1 {
        let g = (generator.sample_at(at(i + 1), 0.0).height_m
            - generator.sample_at(at(i), 0.0).height_m)
            / step_m;
        // Dry ground on both sides: a mountain, not a continental slope.
        if generator.debug_terms(at(i), 0.0)[5] > 0.0
            && generator.debug_terms(at(i + 1), 0.0)[5] > 0.0
        {
            land_grades.push(g.abs());
            if g.abs() > worst.0 {
                worst = (g.abs(), i);
            }
        }
    }
    land_grades.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: usize| land_grades[land_grades.len() * p / 100] * 100.0;
    println!("ON LAND {} samples: median {:.1}%  p90 {:.1}%  p99 {:.1}%  max {:.0}%",
        land_grades.len(), q(50), q(90), q(99), worst.0 * 100.0);
    let i = worst.1;
    println!("steepest on land {:.0}% at sample {i}", worst.0 * 100.0);
    let a = generator.debug_terms(at(i), 0.0);
    let b = generator.debug_terms(at(i + 1), 0.0);
    for k in 0..6 {
        println!("  {:<11} {:>8.1} -> {:>8.1}   grade {:>7.0}%",
            NAMES[k], a[k], b[k], (b[k] - a[k]) / step_m * 100.0);
    }
}
