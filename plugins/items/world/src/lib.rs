//! The items plugin's world half (DECISIONS 111). Items is how a hand places
//! parts, and a hand is in the client: this half offers the world no op of
//! its own. The parts and the ops that change them are the core's.
//!
//! What both halves say once lives here: the plugin's name and version.

use world::{Op, Room, Who};

/// What the world's statement calls the plugin.
pub const NAME: &str = "items";
/// The version of its seam.
pub const VERSION: u32 = 1;

/// The plugin: it holds nothing of a world.
#[derive(Default)]
pub struct Items;

/// The plugin as the server's module holds it.
pub fn plugin() -> Box<dyn world::Plugin> {
    Box::new(Items)
}

impl world::Plugin for Items {
    fn name(&self) -> &'static str {
        NAME
    }

    fn version(&self) -> u32 {
        VERSION
    }

    fn ops(&self) -> Vec<Op> {
        Vec::new()
    }

    fn op(&mut self, _kind: &str, _payload: &[u8], _who: &Who, _room: &mut dyn Room) {}

    fn gone(&mut self, _session: u32) {}
}
