//! The generator: recipe + direction -> terrain.
//!
//! Determinism rules for every line in this crate:
//! - transcendental and rounding math goes through `libm`, never `std`;
//! - only `+ - * /` and comparisons on `f64` otherwise (IEEE exact);
//! - no `mul_add`, no `powi`, no platform intrinsics, no hash maps, no threads.
//!
//! A released generator version never changes behaviour: worlds freeze the
//! version they were created with. New terrain means a new version module
//! beside `v1`, and the golden hashes in `tests/golden.rs` guard the old ones.

mod moon;
mod noise;
mod recipe;
mod v1;
mod v2;

pub use recipe::{Params, Recipe, RecipeError, format_seed, parse_seed};

/// The version new worlds are created with.
pub const GENERATOR_VERSION: u32 = 2;

/// Radius of the moon's datum sphere, metres.
pub const MOON_RADIUS_M: f64 = moon::RADIUS_M;

/// A unit vector from the planet centre.
pub type Direction = [f64; 3];

/// What covers the ground at a sample.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Material {
    Water = 0,
    Sand = 1,
    Grass = 2,
    Forest = 3,
    Rock = 4,
    Snow = 5,
    /// The sea floor below the reach of waves. Generator v2 and later.
    Seabed = 6,
    /// The dust of an airless moon. Generator v2 and later.
    Regolith = 7,
}

/// The terrain under one direction.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sample {
    /// Ground height in metres from the datum sphere. Negative is sea floor.
    pub height_m: f64,
    pub material: Material,
}

impl Sample {
    /// Depth of the sea over this ground, metres. Zero on land.
    pub fn water_depth_m(self) -> f64 {
        (-self.height_m).max(0.0)
    }
}

/// A generator bound to one recipe.
#[derive(Clone, Debug)]
pub struct Generator {
    recipe: Recipe,
    moon_basins: moon::Basins,
}

impl Generator {
    /// Fails when the recipe asks for a generator version this build lacks.
    pub fn new(recipe: Recipe) -> Result<Generator, RecipeError> {
        match recipe.generator_version {
            1 | 2 => Ok(Generator {
                moon_basins: moon::Basins::new(&recipe),
                recipe,
            }),
            version => Err(RecipeError::UnknownGeneratorVersion(version)),
        }
    }

    pub fn recipe(&self) -> &Recipe {
        &self.recipe
    }

    /// Terrain under a unit direction, in full detail. Collision, spawning
    /// and anything saved use this.
    pub fn sample(&self, direction: Direction) -> Sample {
        self.sample_at(direction, 0.0)
    }

    /// The moon's ground under a unit direction from its centre, filtered
    /// like [`Generator::sample_at`]. Worlds older than v2 have a smooth moon.
    pub fn moon_sample_at(&self, direction: Direction, footprint_m: f64) -> Sample {
        match self.recipe.generator_version {
            1 => Sample {
                height_m: 0.0,
                material: Material::Regolith,
            },
            _ => moon::sample(&self.recipe, &self.moon_basins, direction, footprint_m),
        }
    }

    /// Terrain as a mesh with one sample every `footprint_m` metres should see
    /// it: detail finer than the mesh can carry is faded out instead of
    /// aliasing. Presentation only. Generator v1 predates this and ignores it.
    pub fn sample_at(&self, direction: Direction, footprint_m: f64) -> Sample {
        match self.recipe.generator_version {
            1 => v1::sample(&self.recipe, direction),
            _ => v2::sample(&self.recipe, direction, footprint_m),
        }
    }
}
