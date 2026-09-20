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
    /// Share of the planet under water, `0..=1`, approximate.
    pub sea_share: f64,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            relief_m: 1400.0,
            ocean_depth_m: 500.0,
            continent_scale: 1.6,
            sea_share: 0.55,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RecipeError {
    /// A seed is exactly 16 lowercase hex digits.
    InvalidSeed(String),
    UnknownGeneratorVersion(u32),
}

impl fmt::Display for RecipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecipeError::InvalidSeed(s) => {
                write!(f, "invalid seed {s:?}: want 16 lowercase hex digits")
            }
            RecipeError::UnknownGeneratorVersion(v) => write!(f, "unknown generator version {v}"),
        }
    }
}

impl std::error::Error for RecipeError {}

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
        let json = r#"{"seed":"0000000000000001","generator_version":1,"params":{}}"#;
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
}
