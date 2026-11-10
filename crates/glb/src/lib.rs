//! A binary glTF as data: its nodes at rest, the meshes its scene places,
//! their materials and pictures, and what it weighs.
//!
//! Bytes of a `.glb` open into a [`Glb`]. An avatar and a clip read their
//! skin and their animation from it (crate `avatar`); a 3D model reads it at
//! rest as one [`Model`], every mesh where its node stands. [`Glb::weigh`]
//! says what a model would cost to draw from the document and the pictures'
//! headers alone, so a budget is checked before anything is built.
//!
//! This crate reads no file and decodes no picture: a picture stays the
//! bytes the file holds, with the size its header says, for a shell to
//! decode off the frame's thread.

mod model;
mod open;

pub use model::{Alpha, Material, Model, Picture, Run, Vertex, Weight};
pub use open::{Glb, Hierarchy, Transform};

/// Not a binary glTF, or one that cannot be walked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "cannot read glTF: {}", self.0)
    }
}

impl std::error::Error for Error {}
