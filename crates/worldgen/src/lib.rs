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

mod field;
mod moon;
mod noise;
mod plates;
mod recipe;
mod v1;
mod v2;
mod v3;

pub use field::{Field, FieldError, Ground};
pub use recipe::{
    Params, Recipe, RecipeError, Source, format_id, format_seed, parse_id, parse_seed,
};

/// The version new worlds are created with.
pub const GENERATOR_VERSION: u32 = 3;

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

impl Material {
    /// The material a stored byte names, or `None` for a byte no version of
    /// this generator ever wrote.
    pub fn from_id(id: u8) -> Option<Material> {
        Some(match id {
            0 => Material::Water,
            1 => Material::Sand,
            2 => Material::Grass,
            3 => Material::Forest,
            4 => Material::Rock,
            5 => Material::Snow,
            6 => Material::Seabed,
            7 => Material::Regolith,
            _ => return None,
        })
    }
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

/// One direction of a world, from the ground down.
#[derive(Clone, Copy, Debug)]
pub struct Column {
    direction: Direction,
    ground: Sample,
    /// `None` on a generator version that has no caves.
    caves: Option<v3::Column>,
}

impl Column {
    /// The ground this column stands under.
    pub fn ground(self) -> Sample {
        self.ground
    }

    /// How far a height is from the ground, metres, positive inside it. Zero
    /// at exactly [`Column::ground`], so a volume chunk and a heightfield
    /// patch have nothing to reconcile where they meet.
    pub fn density_m(self, height_m: f64) -> f64 {
        match &self.caves {
            None => self.ground.height_m - height_m,
            Some(caves) => v3::density_m(caves, self.direction, height_m),
        }
    }

    /// True when nothing in this column is hollow, so a mesher can take the
    /// ground straight from the height and never ask again.
    pub fn solid(self) -> bool {
        self.caves.is_none_or(|caves| caves.solid())
    }
}

/// A generator bound to one recipe.
#[derive(Clone, Debug)]
pub struct Generator {
    recipe: Recipe,
    moon_basins: moon::Basins,
    plates: plates::Plates,
    field: Option<Field>,
}

impl Generator {
    /// Fails when the recipe asks for a generator version this build lacks,
    /// or when it names a field: that one needs [`Generator::with_field`].
    pub fn new(recipe: Recipe) -> Result<Generator, RecipeError> {
        if let Source::Field(id) = recipe.params.source {
            return Err(RecipeError::FieldRequired(id));
        }
        Generator::build(recipe, None)
    }

    /// The generator of a recipe that names a field, with that field. The id
    /// must be the one the recipe names: a world shaped by other ground is a
    /// different world, and its stored chunks would no longer line up.
    pub fn with_field(recipe: Recipe, field: Field) -> Result<Generator, RecipeError> {
        match recipe.params.source {
            Source::Field(want) if want != field.id() => Err(RecipeError::FieldMismatch {
                want,
                got: field.id(),
            }),
            Source::Field(_) => Generator::build(recipe, Some(field)),
            // A generated world is welcome to ignore the field it was handed.
            Source::Generated => Generator::build(recipe, None),
        }
    }

    /// The id of the field this recipe needs, if it needs one.
    pub fn required_field(recipe: &Recipe) -> Option<[u8; 32]> {
        match recipe.params.source {
            Source::Field(id) => Some(id),
            Source::Generated => None,
        }
    }

    fn build(recipe: Recipe, field: Option<Field>) -> Result<Generator, RecipeError> {
        match recipe.generator_version {
            1..=3 => Ok(Generator {
                moon_basins: moon::Basins::new(&recipe),
                plates: plates::Plates::new(&recipe),
                field,
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

    /// How far a point is from the ground, metres, positive inside it.
    ///
    /// The surface is where this crosses zero, and it crosses at exactly the
    /// height [`Generator::sample_at`] reports, so the volume layer and the
    /// heightfield draw the same ground and the handover between them has
    /// nothing to reconcile.
    ///
    /// This is also the only place a cave can exist: a height has no room for
    /// one. Worlds on generator v1 and v2 are frozen without them, and their
    /// ground is solid all the way down.
    pub fn density_m(&self, direction: Direction, height_m: f64, footprint_m: f64) -> f64 {
        self.column(direction, footprint_m).density_m(height_m)
    }

    /// Everything about a direction that does not change with height, worked
    /// out once.
    ///
    /// A volume walks up a line asking for tens of samples, and the ground
    /// under that line is the same for all of them. This is the difference
    /// between a volume that fits in a frame and one that does not: a column
    /// with no cave in it answers from two numbers.
    pub fn column(&self, direction: Direction, footprint_m: f64) -> Column {
        let ground = self.sample_at(direction, footprint_m);
        let caves = match self.recipe.generator_version {
            1 | 2 => None,
            _ => Some(v3::column(
                &self.recipe,
                direction,
                ground.height_m,
                footprint_m,
            )),
        };
        Column {
            direction,
            ground,
            caves,
        }
    }

    /// Terrain as a mesh with one sample every `footprint_m` metres should see
    /// it: detail finer than the mesh can carry is faded out instead of
    /// aliasing. Presentation only. Generator v1 predates this and ignores it.
    pub fn sample_at(&self, direction: Direction, footprint_m: f64) -> Sample {
        match self.recipe.generator_version {
            1 => v1::sample(&self.recipe, direction),
            2 => v2::sample(&self.recipe, direction, footprint_m),
            _ => v3::sample(
                &self.recipe,
                &self.plates,
                self.field.as_ref(),
                direction,
                footprint_m,
            ),
        }
    }
}
