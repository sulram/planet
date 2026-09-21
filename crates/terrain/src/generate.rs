//! The generated chunk: what the world is where nobody has edited it.
//!
//! One [`worldgen::Column`] per block column, and every cell of that column
//! answered from it. A column with no cave in it answers from two numbers, so
//! the common chunk - all rock, or all sky - costs the 256 columns and almost
//! nothing more.

use topology::{BLOCK_M, QuadSphere, SurfacePoint};
use voxel::{CHUNK_SIDE, Cell, Chunk, Density};
use worldgen::Generator;

use crate::address::ChunkAddr;

/// Metres between samples at full detail: one block.
pub const FOOTPRINT_M: f64 = BLOCK_M;

/// Where a cell's sample sits: the centre of its block.
///
/// Centres, never corners. A corner on the edge of a sector sits exactly on
/// the seam, where folding has no side to choose; a centre is always strictly
/// inside one sector, so the border of a chunk crosses a seam by the same
/// arithmetic as anything else.
pub fn cell_point(addr: ChunkAddr, u: i32, v: i32) -> SurfacePoint {
    let low = addr.low_column();
    SurfacePoint::new(
        low.sector,
        f64::from(low.u) + f64::from(u) + 0.5,
        f64::from(low.v) + f64::from(v) + 0.5,
    )
}

/// Height of a cell's sample above the datum sphere, metres.
pub fn cell_height_m(addr: ChunkAddr, h: i32) -> f64 {
    (f64::from(addr.low_h() + h) + 0.5) * BLOCK_M
}

/// The chunk the recipe puts at an address.
pub fn generate(generator: &Generator, sphere: QuadSphere, addr: ChunkAddr) -> Chunk {
    let blocks = sphere.blocks();
    let side = CHUNK_SIDE as i32;
    let columns: Vec<worldgen::Column> = (0..side * side)
        .map(|i| {
            let point = blocks.wrapped(cell_point(addr, i % side, i / side));
            generator.column(blocks.direction(point), FOOTPRINT_M)
        })
        .collect();

    Chunk::from_fn(|[u, v, h]| {
        let column = columns[u + CHUNK_SIDE * v];
        let density_m = column.density_m(cell_height_m(addr, h as i32));
        Cell {
            density: Density::from_cells(density_m / BLOCK_M),
            material: column.ground().material as voxel::Material,
        }
    })
}
