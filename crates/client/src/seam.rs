//! The command/event seam between the core and any UI.
//!
//! Svelte panels and the native UI send [`Command`]s and render [`Event`]s.
//! Both travel as JSON tagged by `type`, so the same seam serves a WASM
//! boundary, a native panel and tests. Tool logic never leaks past this file.

use scene::Effects;
use serde::{Deserialize, Serialize};
use worldgen::Recipe;

/// How the avatar moves.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Walk,
    Fly,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Regenerate the world from this recipe and respawn.
    SetRecipe {
        recipe: Recipe,
    },
    SetMode {
        mode: Mode,
    },
    /// Wear the avatar at this asset reference: a path under the asset root
    /// (`avatars/Kyle.vrm`) or an absolute URL (a user's own avatar).
    SetAvatar {
        path: String,
    },
    /// Wear any avatar the manifest offers.
    RandomAvatar,
    /// Wear the next avatar on offer, after the current one.
    NextAvatar,
    /// How the picture is made. Fields left out take their default.
    SetEffects {
        effects: Effects,
    },
}

#[derive(Clone, PartialEq, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Sent once, first. `generator_version` is what new worlds should use.
    Ready {
        generator_version: u32,
    },
    RecipeChanged {
        recipe: Recipe,
    },
    ModeChanged {
        mode: Mode,
    },
    /// The avatar now worn. A UI persists this as the person's choice.
    AvatarChanged {
        path: String,
    },
    /// The effects now in force, clamped to what is sane. Sent once at the
    /// start too. A UI persists this as the person's choice.
    EffectsChanged {
        effects: Effects,
    },
    /// Sent about twice a second.
    Stats {
        fps: f32,
        altitude_m: f64,
        speed_mps: f64,
        sector: u8,
    },
    /// A command was refused. `message` is for logs, not for end users.
    Rejected {
        message: String,
    },
}

impl Command {
    pub fn from_json(json: &str) -> Result<Command, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Event {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("events are plain data")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_parse_from_the_documented_json() {
        let json = r#"{"type":"set_recipe","recipe":{"seed":"00000000deadbeef","generator_version":2,"params":{}}}"#;
        assert_eq!(
            Command::from_json(json).unwrap(),
            Command::SetRecipe {
                recipe: Recipe::new(0xdead_beef)
            }
        );
        assert_eq!(
            Command::from_json(r#"{"type":"set_mode","mode":"fly"}"#).unwrap(),
            Command::SetMode { mode: Mode::Fly }
        );
    }

    #[test]
    fn effects_left_out_take_their_default() {
        let command =
            Command::from_json(r#"{"type":"set_effects","effects":{"clouds":false}}"#).unwrap();
        assert_eq!(
            command,
            Command::SetEffects {
                effects: Effects {
                    clouds: false,
                    ..Effects::default()
                }
            }
        );
    }

    #[test]
    fn events_serialize_tagged() {
        assert_eq!(
            Event::ModeChanged { mode: Mode::Walk }.to_json(),
            r#"{"type":"mode_changed","mode":"walk"}"#
        );
    }
}
