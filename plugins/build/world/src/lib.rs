//! The build plugin's world half (DECISIONS 106). Building is how a hand
//! arrives at gestures, and a hand is in the client: this half offers the
//! world no op of its own. The cells and the gesture that changes them are
//! the core's.
//!
//! What both halves of building say once lives here: the plugin's name and
//! version, and its kinds of construction, each as the gestures it comes to.

mod platform;

use world::{Op, Room, Who};

pub use platform::{Base, Platform};

/// What the world's statement calls the plugin.
pub const NAME: &str = "build";
/// The version of its seam.
pub const VERSION: u32 = 1;

/// The sides a platform can have, in cells: powers of two, up to a plot.
pub const PLATFORMS: [u32; 4] = [8, 16, 32, 64];

/// The plugin: it holds nothing of a world.
#[derive(Default)]
pub struct Build;

/// The plugin as the server's module holds it.
pub fn plugin() -> Box<dyn world::Plugin> {
    Box::new(Build)
}

impl world::Plugin for Build {
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
