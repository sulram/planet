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

/// Who hears a line: everyone within `NearBlocks` of the speaker on the same
/// body, or everyone in the world, on every body. Never another world.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Near,
    World,
}

impl Scope {
    pub(crate) fn wire(self) -> protocol::Scope {
        match self {
            Scope::Near => protocol::Scope::Near,
            Scope::World => protocol::Scope::World,
        }
    }

    pub(crate) fn from_wire(scope: protocol::Scope) -> Scope {
        match scope {
            protocol::Scope::Near => Scope::Near,
            protocol::Scope::World => Scope::World,
        }
    }
}

/// What a stroke in a volume does: fill air with the paint, empty cells, or
/// repaint what is solid. One drag is one stroke, however many cells it covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    Create,
    Delete,
    Paint,
}

/// Why a volume could not be opened where the body stands. A front end says
/// it in its own words.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildRefusal {
    /// Volumes stand on the planet.
    Moon,
    /// The ground here is under the sea.
    Sea,
    /// The plot is on the edge of a sector: a volume and its margin stay
    /// inside one.
    Seam,
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
    /// how `--at` aims a headless render, and how a place someone shared in
    /// chat is reached at a click. Walking, flying and, later, portals are
    /// the other ways about a world.
    GoTo {
        place: String,
    },
    /// What to be called: said in Hello and, while online, changed at once.
    /// A signed in person's account name wins on the server; a front end
    /// keeps the account in step itself. Empty is a name too.
    SetName {
        name: String,
    },
    /// Say a line to whoever is in scope. With `here`, where you stand rides
    /// along, filled in by the server from the stance it holds, and comes
    /// back in [`Event::Said`] as a place. Nothing goes out while offline.
    Say {
        scope: Scope,
        text: String,
        #[serde(default)]
        here: bool,
    },
    /// Build with a tool, or stop building with `null`. Starting where no
    /// volume stands opens the one of the plot under the body, on flat ground.
    SetTool {
        tool: Option<Tool>,
    },
    /// The paint the next stroke lays: an index into [`Event::Palette`].
    SetPaint {
        paint: u8,
    },
    /// Takes back the last stroke that landed.
    Undo,
    /// Puts back the last stroke taken back.
    Redo,
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
    /// A line someone said, this client's own included: what the world
    /// heard is what a UI shows. `place` is where the speaker stood when they
    /// shared it, as `GoTo` takes it; `None` when they did not.
    Said {
        session: u32,
        scope: Scope,
        text: String,
        place: Option<String>,
    },
    /// The streamer has nothing left to build for this view: what is on
    /// screen is the world at the detail it is meant to have. Sent each time
    /// that becomes true again, after a new recipe, a leap or a walk.
    Settled,
    /// The tool in hand, `None` when not building, and the paint it lays.
    /// Sent once at the start too.
    ToolChanged {
        tool: Option<Tool>,
        paint: u8,
    },
    /// The paints a cell can take, in order, as `#rrggbb`. Sent once, first
    /// after [`Event::Ready`].
    Palette {
        colors: Vec<String>,
    },
    /// Building was asked for where no volume can be opened.
    BuildRefused {
        reason: BuildRefusal,
    },
    /// Whether there is a stroke to take back and one to put back, whenever
    /// that changes.
    History {
        undo: bool,
        redo: bool,
    },
    /// A command was refused. `message` is for logs, not for end users.
    Rejected {
        message: String,
    },
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
            Event::Said {
                session: 3,
                scope: Scope::Near,
                text: "hi".into(),
                place: None
            }
            .to_json(),
            r#"{"type":"said","session":3,"scope":"near","text":"hi","place":null}"#
        );
    }

    #[test]
    fn a_tool_or_none() {
        assert_eq!(
            Command::from_json(r#"{"type":"set_tool","tool":"paint"}"#).unwrap(),
            Command::SetTool {
                tool: Some(Tool::Paint)
            }
        );
        assert_eq!(
            Command::from_json(r#"{"type":"set_tool","tool":null}"#).unwrap(),
            Command::SetTool { tool: None }
        );
        assert_eq!(
            Event::ToolChanged {
                tool: Some(Tool::Create),
                paint: 3
            }
            .to_json(),
            r#"{"type":"tool_changed","tool":"create","paint":3}"#
        );
    }

    #[test]
    fn a_line_needs_no_here() {
        assert_eq!(
            Command::from_json(r#"{"type":"say","scope":"world","text":"hi"}"#).unwrap(),
            Command::Say {
                scope: Scope::World,
                text: "hi".into(),
                here: false
            }
        );
    }
}
