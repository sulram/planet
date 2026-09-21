//! Where a chunk is, how coarse it is, and which cells it holds.
//!
//! A chunk of level `L` holds cells `2^L` blocks wide. Its cells live on the
//! block grid coarsened by `L`, and the chunks themselves on that grid
//! coarsened again by [`voxel::CHUNK_BITS`]. So a chunk's neighbours across a
//! seam are the block grid's neighbours at every level, and nothing here has
//! to know what a seam is
//! (`topology::a_coarsened_grid_keeps_its_seams`).
//!
//! Level 0 is the ground you stand on, one cell a block. The coarsest level a
//! body has is the one whose chunk grid is a single chunk per sector face: six
//! chunks for the whole world, which is what it is drawn from at a distance.

use topology::{BLOCK_M, Column, Grid, QuadSphere};
use voxel::{CHUNK_BITS, CHUNK_SIDE};

/// How coarse a chunk is: its cells are `2^0.. ` blocks wide.
pub type Level = u32;

/// One chunk of a body: how coarse, which cell of its chunk grid, and how far
/// up.
///
/// `h` counts chunks of this level from the datum sphere, so a coarse chunk
/// spans more of the world vertically as well as across.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChunkAddr {
    pub level: Level,
    pub column: Column,
    pub h: i16,
}

impl ChunkAddr {
    pub fn new(level: Level, column: Column, h: i16) -> ChunkAddr {
        ChunkAddr { level, column, h }
    }

    /// Blocks across one of this chunk's cells.
    pub fn cell_blocks(self) -> i32 {
        1 << self.level
    }

    /// Metres across one of this chunk's cells.
    pub fn cell_m(self) -> f64 {
        BLOCK_M * f64::from(self.cell_blocks() as u32)
    }

    /// Metres along one edge of this chunk.
    pub fn span_m(self) -> f64 {
        self.cell_m() * CHUNK_SIDE as f64
    }

    /// The cell column of this chunk's low corner, on its own level's grid.
    pub fn low_column(self) -> Column {
        Column::new(
            self.column.sector,
            self.column.u << CHUNK_BITS,
            self.column.v << CHUNK_BITS,
        )
    }

    /// The height of this chunk's low cell, in cells of its own level.
    pub fn low_h(self) -> i32 {
        i32::from(self.h) * CHUNK_SIDE as i32
    }
}

/// The grid one cell of `level` lives on.
pub fn cell_grid(sphere: QuadSphere, level: Level) -> Option<Grid> {
    sphere.blocks().coarsened(level)
}

/// The grid chunks of `level` live on.
pub fn chunk_grid(sphere: QuadSphere, level: Level) -> Option<Grid> {
    cell_grid(sphere, level)?.coarsened(CHUNK_BITS)
}

/// The coarsest level this body has: one chunk per sector face, six for the
/// whole world. Drawing it from further away than that would mean a cell
/// wider than a face, which the grid has no room for.
pub fn coarsest(sphere: QuadSphere) -> Level {
    sphere.bits() - CHUNK_BITS
}

/// How many chunks of `level` thick half the build band is.
///
/// The band follows the ground, not the datum (ARCHITECTURE, Topology): it is
/// `+-band_blocks` around the surface, so a column under a mountain and a
/// column under a trench hold their chunks at different heights. A chunk that
/// only clips the band counts, because the band has to be whole.
pub fn band_chunks(sphere: QuadSphere, level: Level) -> i16 {
    let band = i32::from(sphere.band_blocks()) >> level;
    let side = CHUNK_SIDE as i32;
    (((band + side - 1) / side) as i16).max(1)
}
