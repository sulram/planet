//! A world shaped by the Earth field has to be the Earth.
//!
//! Skipped when the field has not been baked (`bun run field`): it is 25 MB
//! and gitignored, so CI runs without it. When it is there, this is what
//! catches a mirrored, rotated or upside down bake, which every other test in
//! the crate is blind to because it only ever compares the generator with
//! itself.

use worldgen::{Field, Generator, Recipe, Source};

const FIELD: &str = "../../assets/fields/earth.field";

/// The generator for a world over the baked Earth, if this checkout has one.
fn earth() -> Option<Generator> {
    let bytes = std::fs::read(FIELD).ok()?;
    let field = Field::parse(bytes).expect("a baked field");
    let mut recipe = Recipe::new(1);
    recipe.params.source = Source::Field(field.id());
    Some(Generator::with_field(recipe, field).expect("the field the recipe names"))
}

/// The unit vector our world puts a latitude and longitude at. North is `+y`
/// and the prime meridian is `+x`; east turns toward `-z`, because the frame
/// the engine draws in is a mirror of the body's and a map baked straight in
/// comes out east for west. The sign lives in `scripts/field.ts` as `EAST`.
///
/// It is the same sign the bake uses, so this file cannot be what checks it:
/// sharing a convention with the bake is exactly what makes it agree. What it
/// does catch is the bake and the reading drifting apart, which is what put
/// the first bake upside down. Only looking at the planet catches the sign.
fn at(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    [lat.cos() * lon.cos(), lat.sin(), -lat.cos() * lon.sin()]
}

#[test]
fn land_is_where_the_earth_has_land() {
    let Some(generator) = earth() else {
        eprintln!("no field baked: skipping (bun run field)");
        return;
    };
    // Pairs that a mirror would swap: each is well inland or well out to sea,
    // and the other is on the opposite side of the same parallel.
    for (name, lat, lon, dry) in [
        ("Cairo", 30.0, 31.0, true),
        ("Atlantic west of Cairo", 30.0, -31.0, false),
        ("Amazon", -3.0, -62.0, true),
        ("Indian Ocean east of it", -3.0, 62.0, false),
        ("Mongolia", 47.0, 104.0, true),
        ("Atlantic west of it", 47.0, -34.0, false),
        ("Australia", -25.0, 134.0, true),
        ("South Atlantic", -25.0, -14.0, false),
        ("Greenland", 72.0, -40.0, true),
        ("Antarctica", -80.0, 20.0, true),
        ("Arctic Ocean", 88.0, 0.0, false),
        ("Pacific", 0.0, -150.0, false),
    ] {
        let height_m = generator.sample(at(lat, lon)).height_m;
        assert_eq!(
            height_m > 0.0,
            dry,
            "{name} ({lat}, {lon}) came out at {height_m:.0} m"
        );
    }
}

#[test]
fn the_poles_are_where_the_poles_are() {
    let Some(generator) = earth() else {
        return;
    };
    // The south pole stands on two kilometres of ice; the north pole floats.
    assert!(generator.sample(at(-89.5, 0.0)).height_m > 0.0);
    assert!(generator.sample(at(89.5, 0.0)).height_m < 0.0);
}
