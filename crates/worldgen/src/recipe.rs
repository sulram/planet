//! The recipe: everything needed to regenerate a world's untouched terrain.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::GENERATOR_VERSION;

/// Seed + params + generator version.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Recipe {
    /// Written as 16 lowercase hex digits everywhere outside Rust: JSON
    /// numbers cannot hold a `u64`.
    #[serde(with = "seed_hex")]
    pub seed: u64,
    pub generator_version: u32,
    #[serde(default)]
    pub params: Params,
}

impl Recipe {
    /// A recipe for the current generator with default params.
    pub fn new(seed: u64) -> Recipe {
        Recipe {
            seed,
            generator_version: GENERATOR_VERSION,
            params: Params::default(),
        }
    }
}

/// What gives a world its shape: its seed alone, or a baked field.
///
/// Both still take the seed. A field carries the low frequencies of some real
/// body, and everything under one of its texels is the generator's, so one
/// field is a family of worlds that share a coastline, never a single world.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Tectonic plates over the seed. Generator v3 and later.
    #[default]
    Generated,
    /// A baked field, named by the content id the bake wrote into it.
    Field(#[serde(with = "id_hex")] [u8; 32]),
}

/// The knobs of generator v1. Absent fields take their defaults, so `{}` is a
/// complete params object.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Params {
    /// Height of the tallest peaks, metres.
    pub relief_m: f64,
    /// Depth of the deepest sea floor, metres.
    pub ocean_depth_m: f64,
    /// Continent sized features per planet radius. Higher is more, smaller land.
    pub continent_scale: f64,
    /// Share of the planet under water, `0..=1`, approximate. A field fixes
    /// its own coastline, so this and `continent_scale` do nothing there.
    pub sea_share: f64,
    /// Where the shape comes from. Generator v3 and later.
    pub source: Source,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            relief_m: 1400.0,
            ocean_depth_m: 500.0,
            continent_scale: 1.6,
            sea_share: 0.55,
            source: Source::Generated,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RecipeError {
    /// A seed is exactly 16 lowercase hex digits.
    InvalidSeed(String),
    UnknownGeneratorVersion(u32),
    /// The recipe names a field and none was handed to the generator.
    FieldRequired([u8; 32]),
    /// The field handed over is not the one the recipe names.
    FieldMismatch {
        want: [u8; 32],
        got: [u8; 32],
    },
}

impl fmt::Display for RecipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecipeError::InvalidSeed(s) => {
                write!(f, "invalid seed {s:?}: want 16 lowercase hex digits")
            }
            RecipeError::UnknownGeneratorVersion(v) => write!(f, "unknown generator version {v}"),
            RecipeError::FieldRequired(id) => {
                write!(f, "this recipe needs field {}", format_id(id))
            }
            RecipeError::FieldMismatch { want, got } => write!(
                f,
                "wrong field: want {}, got {}",
                format_id(want),
                format_id(got)
            ),
        }
    }
}

impl std::error::Error for RecipeError {}

/// A field id as 64 lowercase hex digits.
pub fn format_id(id: &[u8; 32]) -> String {
    let mut text = String::with_capacity(64);
    for byte in id {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// Reads the form [`format_id`] writes, and no other.
pub fn parse_id(text: &str) -> Result<[u8; 32], RecipeError> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return Err(RecipeError::InvalidSeed(text.to_owned()));
    }
    let mut id = [0u8; 32];
    for (index, pair) in bytes.chunks_exact(2).enumerate() {
        let digit = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        };
        match (digit(pair[0]), digit(pair[1])) {
            (Some(hi), Some(lo)) => id[index] = hi * 16 + lo,
            _ => return Err(RecipeError::InvalidSeed(text.to_owned())),
        }
    }
    Ok(id)
}

mod id_hex {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(id: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::format_id(id))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let text = String::deserialize(d)?;
        super::parse_id(&text).map_err(D::Error::custom)
    }
}

pub fn format_seed(seed: u64) -> String {
    format!("{seed:016x}")
}

pub fn parse_seed(text: &str) -> Result<u64, RecipeError> {
    let valid = text.len() == 16 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if !valid {
        return Err(RecipeError::InvalidSeed(text.to_owned()));
    }
    u64::from_str_radix(text, 16).map_err(|_| RecipeError::InvalidSeed(text.to_owned()))
}

mod seed_hex {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(seed: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::format_seed(*seed))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let text = String::deserialize(d)?;
        super::parse_seed(&text).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_round_trips_as_hex() {
        let recipe = Recipe::new(0x0000_0000_dead_beef);
        let json = serde_json::to_string(&recipe).unwrap();
        assert!(json.contains(r#""seed":"00000000deadbeef""#), "{json}");
        assert_eq!(serde_json::from_str::<Recipe>(&json).unwrap(), recipe);
    }

    #[test]
    fn empty_params_are_defaults() {
        let json = r#"{"seed":"0000000000000001","generator_version":3,"params":{}}"#;
        assert_eq!(
            serde_json::from_str::<Recipe>(json).unwrap(),
            Recipe::new(1)
        );
    }

    #[test]
    fn bad_seeds_are_rejected() {
        for bad in [
            "",
            "1",
            "00000000DEADBEEF",
            "00000000deadbeeg",
            "+000000000000001",
        ] {
            assert!(parse_seed(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_generated_source_is_the_default_and_stays_out_of_the_way() {
        let json = serde_json::to_string(&Params::default()).unwrap();
        assert!(json.contains(r#""source":"generated""#), "{json}");
    }

    #[test]
    fn a_field_source_round_trips_by_id() {
        let id = [0xabu8; 32];
        let params = Params {
            source: Source::Field(id),
            ..Params::default()
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(
            json.contains(&format!(r#""field":"{}""#, format_id(&id))),
            "{json}"
        );
        assert_eq!(serde_json::from_str::<Params>(&json).unwrap(), params);
    }

    #[test]
    fn ids_round_trip_and_bad_ones_are_rejected() {
        let id = [
            0u8, 1, 2, 250, 255, 16, 32, 64, 128, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31,
            33, 35, 37, 39, 41, 43, 45, 47, 49, 51,
        ];
        assert_eq!(parse_id(&format_id(&id)).unwrap(), id);
        for bad in ["", "ab", &"A".repeat(64), &"z".repeat(64)] {
            assert!(parse_id(bad).is_err(), "{bad:?}");
        }
    }
}
