//! The command/event seam between the core and any UI.
//!
//! Svelte panels and the native UI send [`Command`]s and render [`Event`]s.
//! Both travel as JSON tagged by `type`, so the same seam serves a WASM
//! boundary, a native panel and tests.
//!
//! A plugin's commands and events ride the same seam under its name: a
//! `type` with a dot in it, `chat.say`, is a plugin's, and the host hands it
//! over (`crate::plugin`).

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

/// What a session may do, as the world said it in its welcome. The order is
/// the order of trust: a builder builds, an admin builds and founds the world.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Anonymous,
    SignedIn,
    Builder,
    Admin,
}

impl Level {
    /// A level this client has yet to learn reads as the least.
    pub(crate) fn from_wire(level: i32) -> Level {
        match protocol::Level::try_from(level) {
            Ok(protocol::Level::SignedIn) => Level::SignedIn,
            Ok(protocol::Level::Builder) => Level::Builder,
            Ok(protocol::Level::Admin) => Level::Admin,
            Ok(protocol::Level::Anonymous) | Err(_) => Level::Anonymous,
        }
    }
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
    /// Stand where a pose says, and look the way it says: `"4-K7M42Q"`, or
    /// `"m4-K7M42Q@40,180,-5"` for a body on the moon, forty blocks up,
    /// facing south and looking a little down. A code shorter than full
    /// precision names a box, and the middle of it is where you land.
    ///
    /// Arrival and travel both: how a shared address opens where it says,
    /// how `--at` aims a headless render, and how a place someone shared is
    /// reached at a click. Walking, flying and, later, portals are the other
    /// ways about a world.
    GoTo {
        place: String,
    },
    /// What to be called: said in Hello and, while online, changed at once.
    /// A signed in person's account name wins on the server; a front end
    /// keeps the account in step itself. Empty is a name too.
    SetName {
        name: String,
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
        /// Where the body is, as a person says it: `"4-K7M42Q"`, or
        /// `"m4-K7M42Q@40"` on the moon, off the ground. The sector is a
        /// character of it, so it is not sent twice.
        place: String,
        /// The same, plus the way of looking: what a link carries, so someone
        /// who opens it stands where you stood seeing what you saw. A front
        /// end puts this in the address bar and hands it back as `GoTo`
        /// without ever taking it apart.
        pose: String,
        /// Degrees clockwise from north, `0..360`. `None` at a pole, where a
        /// bearing is not a thing that exists. The letters are the front
        /// end's: N and S are English, and a user visible string belongs to
        /// whoever holds the locale.
        bearing_deg: Option<f64>,
    },
    /// The link to the world server, whenever it changes. Sent once at the
    /// start too, offline.
    Session {
        status: SessionStatus,
        /// This client's session in the world, while online.
        session: Option<u32>,
        /// What the world said this session may do. `None` until a world
        /// has spoken, and then the last word stands through a dropped link.
        level: Option<Level>,
    },
    /// Who else is here, whenever that changes. Empty when offline.
    Peers {
        peers: Vec<PeerInfo>,
    },
    /// Where every head in view is on the screen, this client's own
    /// included: sent every frame while there is one, and once empty after
    /// the last. A front end hangs a nametag or a balloon there.
    Anchors {
        anchors: Vec<Anchor>,
    },
    /// The plugins that are on in the world this client is in, whenever
    /// that changes: what a front end mounts. Empty when offline, where no
    /// world has spoken.
    Statement {
        plugins: Vec<PluginOn>,
    },
    /// The streamer has nothing left to build for this view: what is on
    /// screen is the world at the detail it is meant to have. Sent each time
    /// that becomes true again, after a new recipe, a leap or a walk.
    Settled,
    /// The paints a cell can take, in order, as `#rrggbb`. Sent once, first
    /// after [`Event::Ready`].
    Palette {
        colors: Vec<String>,
    },
    /// A command was refused. `message` is for logs, not for end users.
    Rejected {
        message: String,
    },
    /// An event of a plugin, as the plugin wrote it: an object whose `type`
    /// is the plugin's name and the event's, `chat.said`.
    #[serde(untagged)]
    Plugin(serde_json::Value),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Offline,
    /// The socket is open and Welcome is awaited.
    Connecting,
    Online,
}

/// Someone else in the world, as a front end lists them. The name is the
/// person's own or empty; a visitor is signed out.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct PeerInfo {
    pub session: u32,
    pub name: String,
    pub visitor: bool,
}

/// A plugin that is on in this world, as the world's statement says it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct PluginOn {
    pub name: String,
    pub version: u32,
}

/// A head on the screen: fractions of the viewport from the top left, so
/// no front end needs to know the canvas size, and how far the head is.
#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub struct Anchor {
    pub session: u32,
    pub x: f32,
    pub y: f32,
    pub distance_m: f32,
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
        let json = r#"{"type":"set_recipe","recipe":{"seed":"00000000deadbeef","generator_version":3,"params":{}}}"#;
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
        assert_eq!(
            Event::Statement {
                plugins: vec![PluginOn {
                    name: "chat".into(),
                    version: 1
                }]
            }
            .to_json(),
            r#"{"type":"statement","plugins":[{"name":"chat","version":1}]}"#
        );
    }

    #[test]
    fn a_plugins_event_goes_out_as_the_plugin_wrote_it() {
        let event = Event::Plugin(serde_json::json!({"type": "chat.said", "text": "hi"}));
        assert_eq!(event.to_json(), r#"{"text":"hi","type":"chat.said"}"#);
    }
}
