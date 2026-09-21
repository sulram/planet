//! Quad sphere topology: six square sectors projected on a sphere.
//!
//! Two views of the same place:
//! - **address space**: integers, `(sector, u, v, h)`. Every block is a unit
//!   cube. Simulation and storage live here.
//! - **world space**: `f64` metres from the planet centre. Rendering and
//!   flight live here.
//!
//! Two levels of structure. A [`Grid`] is six square faces at one grain, and
//! owns everything that is pure arithmetic: seams, neighbours, the warp. A
//! [`QuadSphere`] is one body: a block grid plus the radius that turns cells
//! into metres. A chunk grid is the block grid coarsened, so it inherits the
//! seams rather than repeating them.
//!
//! All transcendental math goes through `libm`, so results are bit identical
//! on native, WASM and ARM (the generator depends on it).

mod address;
mod grid;
mod project;
mod quad_sphere;
mod sector;
mod surface;
pub mod vec3;

pub use address::{Address, Column, Step};
pub use project::Tangents;
pub use grid::{Grid, MAX_BITS};
pub use quad_sphere::{MIN_BITS, QuadSphere};
pub use sector::{Dir, Sector};
pub use surface::SurfacePoint;
pub use vec3::Vec3;

/// Edge of one block at a sector centre, in metres.
pub const BLOCK_M: f64 = 0.5;
