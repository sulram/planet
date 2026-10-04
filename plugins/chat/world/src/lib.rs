//! The chat plugin's world half (DECISIONS 69, 99): a line said is relayed to
//! everyone in its scope and never stored. It owns no state of the world,
//! only how often each session spoke lately.
//!
//! What both halves of chat say once lives here: the plugin's name and
//! version, its op and its event, its wire and its limits.

use core::time::Duration;
use std::collections::HashMap;

use prost::Message;
use world::{Level, Moment, Op, Room, Who};

/// The plugin's wire, generated from the schema in its `wire/` by
/// `bun run proto`.
pub mod wire {
    #![allow(clippy::all, clippy::pedantic)]
    include!("gen/planet/chat/v1/planet.chat.v1.rs");
}

/// What the envelope and the world's statement call the plugin.
pub const NAME: &str = "chat";
/// The version of its wire and of its seam.
pub const VERSION: u32 = 1;

/// The op a session asks, and the event everyone in scope hears.
pub const SAY: &str = "say";
pub const SAID: &str = "said";

/// A line is at most this many characters, counted as a person counts them
/// (code points, never bytes). Past it the line is dropped and nobody is
/// told: a UI holds the same limit, so a person never meets it, and only a
/// client that ignores it does. The wire's worst case is four bytes a
/// character.
pub const LINE_CHARS: usize = 500;

/// A session says at most this many lines in [`LINE_WINDOW`]. One more is
/// dropped unheard.
pub const LINE_BURST: usize = 5;
pub const LINE_WINDOW: Duration = Duration::from_secs(5);

/// How far a line said near reaches: the distance between two stances on the
/// same body, in blocks, along the datum and up. One constant for every world
/// until a world asks for its own.
pub const NEAR_BLOCKS: f64 = 64.0;

/// The plugin. The host calls it one call at a time.
#[derive(Default)]
pub struct Chat {
    /// When each session said its last [`LINE_BURST`] lines, oldest first.
    said: HashMap<u32, [Option<Moment>; LINE_BURST]>,
}

/// The plugin as the server's module holds it.
pub fn plugin() -> Box<dyn world::Plugin> {
    Box::new(Chat::default())
}

impl world::Plugin for Chat {
    fn name(&self) -> &'static str {
        NAME
    }

    fn version(&self) -> u32 {
        VERSION
    }

    /// Anyone in the world may speak, a visitor included. A mute is an answer
    /// over the permission hook.
    fn ops(&self) -> Vec<Op> {
        vec![Op {
            kind: SAY,
            level: Level::Anonymous,
        }]
    }

    /// Relays a line to everyone in its scope, the speaker included, so what
    /// a client shows is what the world heard. The speaker's place rides
    /// along when asked for, from the stance the room holds and never from
    /// the client's word.
    fn op(&mut self, kind: &str, payload: &[u8], who: &Who, room: &mut dyn Room) {
        if kind != SAY {
            return;
        }
        let Ok(say) = wire::Say::decode(payload) else {
            return;
        };
        let text = say.text.trim();
        if text.chars().count() > LINE_CHARS || (text.is_empty() && !say.here) {
            return;
        }
        if !self.may_speak(who.session, room.now()) {
            return;
        }
        let said = wire::Said {
            session: who.session,
            scope: say.scope,
            text: text.to_owned(),
            stance: if say.here { who.stance } else { None },
        };
        let everyone = say.scope == i32::from(wire::Scope::World);
        let measure = room.measure();
        let near = |other: &Who| match (&who.stance, &other.stance) {
            (Some(a), Some(b)) => measure
                .apart(a, b)
                .is_some_and(|blocks| blocks <= NEAR_BLOCKS),
            _ => false,
        };
        let to: Vec<u32> = room
            .sessions()
            .iter()
            .filter(|other| everyone || other.session == who.session || near(other))
            .map(|other| other.session)
            .collect();
        room.tell(SAID, said.encode_to_vec(), &to);
    }

    /// Forgets how often a session spoke.
    fn gone(&mut self, session: u32) {
        self.said.remove(&session);
    }
}

impl Chat {
    /// The rate limit: true, and the line counted, unless [`LINE_BURST`]
    /// lines were already said within [`LINE_WINDOW`].
    fn may_speak(&mut self, session: u32, now: Moment) -> bool {
        let said = self.said.entry(session).or_default();
        if said[0].is_some_and(|oldest| now.since(oldest) < LINE_WINDOW) {
            return false;
        }
        said.rotate_left(1);
        said[LINE_BURST - 1] = Some(now);
        true
    }
}
