//! Avatars: VRM models and the humanoid clips that move them.
//!
//! Pure data in, pure data out. Bytes of a `.vrm` become an [`Avatar`]; bytes
//! of a Mixamo rigged `.glb` become a [`Clip`] retargeted to the VRM humanoid,
//! so every avatar shares every clip. An [`Animator`] blends clips into a pose
//! and the avatar turns the pose into joint matrices for a renderer.
//!
//! Scope, on purpose: VRM 0.x (what the default set is), one skin, PNG
//! textures. Spring bones, expressions and look-at are not read.

mod bones;
mod clip;
mod glb;
mod model;

pub use clip::{Animator, Clip, Pose};
pub use model::Avatar;

#[derive(Debug)]
pub enum Error {
    /// Not a binary glTF, or one we cannot walk.
    Gltf(String),
    /// A glTF, but not a VRM 0.x humanoid.
    NotVrm(String),
    /// A glTF, but no Mixamo rigged animation inside.
    NotClip(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Gltf(why) => write!(f, "cannot read glTF: {why}"),
            Error::NotVrm(why) => write!(f, "not a usable VRM: {why}"),
            Error::NotClip(why) => write!(f, "not a usable clip: {why}"),
        }
    }
}

impl std::error::Error for Error {}
