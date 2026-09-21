//! The generated chunk: what the world is where nobody has edited it.
//!
//! One [`worldgen::Column`] per cell column, and every cell of that column
//! answered from it. A column with no cave in it answers from two numbers, so
//! the common chunk - all rock, or all sky - costs the 256 columns and almost
//! nothing more.
//!
//! A coarse chunk is generated, not built from the chunks under it: the
//! generator is asked at the footprint of its cells, so detail finer than the
//! chunk can carry is faded out rather than sampled and aliased (DECISIONS 29,
//! 43). That is the cheap half of the pyramid; keeping the mean is the other.

use topology::{QuadSphere, SurfacePoint};
use voxel::{CHUNK_SIDE, Cell, Chunk, Density};
use worldgen::Generator;

use crate::address::{ChunkAddr, cell_grid};

/// Where a cell's sample sits: the centre of its own cell.
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
    (f64::from(addr.low_h() + h) + 0.5) * addr.cell_m()
}

/// The chunk the recipe puts at an address.
pub fn generate(generator: &Generator, sphere: QuadSphere, addr: ChunkAddr) -> Chunk {
    let cells = cell_grid(sphere, addr.level).expect("a level this body has");
    let side = CHUNK_SIDE as i32;
    let footprint_m = addr.cell_m();
    let columns: Vec<worldgen::Column> = (0..side * side)
        .map(|i| {
            let point = cells.wrapped(cell_point(addr, i % side, i / side));
            generator.column(cells.direction(point), footprint_m)
        })
        .collect();

    Chunk::from_fn(|[u, v, h]| {
        let column = columns[u + CHUNK_SIDE * v];
        let density_m = column.density_m(cell_height_m(addr, h as i32));
        Cell {
            // In cells of this level, so a coarse chunk's surface sits where
            // its own grid says and the mesher needs no scale of its own.
            density: Density::from_cells(density_m / footprint_m),
            material: column.ground().material as voxel::Material,
        }
    })
}
