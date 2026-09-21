//! Voxel ground around a body: the read path, the mesher and the streamer.
//!
//! The ground is a volume everywhere, not a surface with a window cut into it
//! (DECISIONS 48). A chunk is the unit of storage, of streaming and of
//! meshing, and the grid chunks live on is the block grid coarsened, so a
//! chunk crossing a seam is the same arithmetic as a block crossing one.
//!
//! No GPU and no IO: this crate turns a recipe into [`scene::TerrainMesh`]es
//! and says which ones a renderer should be holding.

mod address;
mod generate;
mod mesh;
mod read;
mod stream;

pub use address::{ChunkAddr, band_chunks, band_h, chunk_grid};
pub use generate::{FOOTPRINT_M, generate};
pub use mesh::mesh;
pub use read::{Chunks, Lattice};
pub use stream::{BUDGET, CHUNK_M, REACH_M, Terrain};
