//! The chat plugin's client half (DECISIONS 69): a line said goes up in the
//! core's envelope, a line heard comes over the seam with the speaker's
//! place as a person reads it. A line is never kept here: a front end holds
//! what it shows.
//!
//! Over the seam, JSON tagged by `type`:
//!
//! ```json
//! {"type":"chat.say","scope":"near","text":"hi","here":true}
//! {"type":"chat.said","session":2,"scope":"near","text":"hi","place":"4-K7M42Q"}
//! ```

use chat_world::{SAID, SAY};
use client::{Host, Plugin};
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The plugin's name, its version and its wire are said once, by its world
/// half (DECISIONS 101).
pub use chat_world::{NAME, VERSION, wire};

/// Who hears a line: everyone within reach of the speaker on the same body,
/// or everyone in the world, on every body. Never another world.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Near,
    World,
}

impl Scope {
    fn wire(self) -> wire::Scope {
        match self {
            Scope::Near => wire::Scope::Near,
            Scope::World => wire::Scope::World,
        }
    }

    fn from_wire(scope: wire::Scope) -> Scope {
        match scope {
            wire::Scope::Near => Scope::Near,
            wire::Scope::World => Scope::World,
        }
    }
}

/// `chat.say`: a line for whoever is in scope. With `here`, where you stand
/// rides along, filled in by the server from the stance it holds, and comes
/// back in [`Said`] as a place. Nothing goes out while offline.
#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct Say {
    pub scope: Scope,
    pub text: String,
    #[serde(default)]
    pub here: bool,
}

/// `chat.said`: a line someone said, this client's own included, so what the
/// world heard is what a UI shows. `place` is where the speaker stood when
/// they shared it, as `GoTo` takes it; `None` when they did not.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct Said {
    pub session: u32,
    pub scope: Scope,
    pub text: String,
    pub place: Option<String>,
}

/// The plugin: it holds nothing between two lines.
#[derive(Default)]
pub struct Chat;

/// The plugin as a shell plugs it in.
pub fn plugin() -> Box<dyn Plugin> {
    Box::new(Chat)
}

impl Plugin for Chat {
    fn name(&self) -> &'static str {
        NAME
    }

    fn version(&self) -> u32 {
        VERSION
    }

    fn command(&mut self, kind: &str, body: Value, host: &mut Host<'_>) {
        if kind != SAY {
            return host.reject(format!("`{kind}` is no command of chat"));
        }
        match serde_json::from_value::<Say>(body) {
            Ok(say) => host.send(
                SAY,
                wire::Say {
                    scope: say.scope.wire().into(),
                    text: say.text,
                    here: say.here,
                }
                .encode_to_vec(),
            ),
            Err(error) => host.reject(error.to_string()),
        }
    }

    fn receive(&mut self, kind: &str, payload: &[u8], host: &mut Host<'_>) {
        if kind != SAID {
            return;
        }
        match wire::Said::decode(payload) {
            Ok(said) => {
                let place = said.stance.as_ref().and_then(|stance| host.place(stance));
                host.emit(
                    SAID,
                    &Said {
                        session: said.session,
                        scope: Scope::from_wire(said.scope()),
                        text: said.text,
                        place,
                    },
                );
            }
            Err(error) => host.reject(format!("a line from the server: {error}")),
        }
    }
}
