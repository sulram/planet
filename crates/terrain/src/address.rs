//! Where a chunk is, and which cells it holds.
//!
//! A chunk grid is the block grid coarsened by [`voxel::CHUNK_BITS`], so a
//! chunk's neighbours across a seam are the block grid's neighbours and
//! nothing here needs to know what a seam is
//! (`topology::a_coarsened_grid_keeps_its_seams`).

use topology::{Column, Grid, QuadSphere};
use voxel::{CHUNK_BITS, CHUNK_SIDE};

/// One chunk of a body: a cell of the chunk grid, and how far up it sits.
///
/// `h` counts chunks from the datum sphere, so chunk `0` straddles `h = 0`
/// and the band is a handful of them either way.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChunkAddr {
    pub column: Column,
    pub h: i16,
}

impl ChunkAddr {
    pub fn new(column: Column, h: i16) -> ChunkAddr {
        ChunkAddr { column, h }
    }

    /// The block column of this chunk's low corner, on the block grid.
    pub fn low_column(self) -> Column {
        Column::new(
            self.column.sector,
            self.column.u << CHUNK_BITS,
            self.column.v << CHUNK_BITS,
        )
    }

    /// The block height of this chunk's low cell.
    pub fn low_h(self) -> i32 {
        i32::from(self.h) * CHUNK_SIDE as i32
    }
}

/// The chunk grid of a body.
pub fn chunk_grid(sphere: QuadSphere) -> Grid {
    sphere
        .blocks()
        .coarsened(CHUNK_BITS)
        .expect("a body is at least one chunk per sector side")
}

/// How many chunks thick half the build band is.
///
/// The band follows the ground, not the datum (ARCHITECTURE, Topology): it is
/// `+-band_blocks` around the surface, so a column under a mountain and a
/// column under a trench hold their chunks at different heights. A chunk that
/// only clips the band counts, because the band has to be whole.
pub fn band_chunks(sphere: QuadSphere) -> i16 {
    let band = i32::from(sphere.band_blocks());
    let side = CHUNK_SIDE as i32;
    ((band + side - 1) / side) as i16
}

/// The chunk heights the band covers under one column, given where its ground
/// is in blocks from the datum. Inclusive at both ends.
pub fn band_h(sphere: QuadSphere, ground_h: i32) -> core::ops::RangeInclusive<i16> {
    let side = CHUNK_SIDE as i32;
    let middle = ground_h.div_euclid(side) as i16;
    let reach = band_chunks(sphere);
    (middle - reach)..=(middle + reach)
}
