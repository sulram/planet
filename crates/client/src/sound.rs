//! What is heard: every source of sound in the world this frame, where it
//! stands and how far it carries. The shell mixes them: one listener at the
//! eye, a panner a source, with the platform's own audio. A picture that
//! moves and carries sound is the first source; a peer's voice, heard by
//! proximity, is the next, over the same list.

use glam::DVec3;
use part::PartId;
use scene::PictureId;

/// Who makes a sound.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SoundSource {
    /// A playing picture: the shell holds its video, and the sound is its.
    Picture(PictureId),
    /// A sound placed in the world: the shell plays its file.
    Clip(PartId),
}

/// One source of sound this frame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sound {
    pub source: SoundSource,
    /// Where it stands, in world space.
    pub position: DVec3,
    /// How far it is heard, metres: silent from there on.
    pub reach_m: f64,
}
