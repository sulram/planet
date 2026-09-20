//! Quad sphere topology: six square sectors projected on a sphere.
//!
//! Two views of the same place:
//! - **address space**: integers, `(sector, u, v, h)`. Every block is a unit
//!   cube. Simulation and storage live here.
//! - **world space**: `f64` metres from the planet centre. Rendering and
//!   flight live here.
//!
//! All transcendental math goes through `libm`, so results are bit identical
//! on native, WASM and ARM (the generator depends on it).

mod address;
mod project;
mod sector;
mod surface;
pub mod vec3;

pub use address::{Address, Column, Step};
pub use project::Tangents;
pub use sector::{Dir, Sector};
pub use surface::SurfacePoint;
pub use vec3::Vec3;

/// Blocks per sector side, as a power of two: `u` and `v` fit in 16 bits.
pub const SECTOR_BITS: u32 = 16;
/// Blocks per sector side.
pub const SECTOR_SIDE: u32 = 1 << SECTOR_BITS;
/// Edge of one block, in metres.
pub const BLOCK_M: f64 = 0.5;
/// Radius of the datum sphere (`h = 0`), in metres. Four sector sides make
/// one great circle.
pub const RADIUS_M: f64 = SECTOR_SIDE as f64 * BLOCK_M * 4.0 / core::f64::consts::TAU;
