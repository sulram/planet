//! A body: one block grid, and the radius that turns cells into metres.
//!
//! The size is not a constant. The planet, a moon and a world made to try the
//! engine out are the same topology with different numbers, so every question
//! about metres is asked of a [`QuadSphere`] and not of the crate.
//!
//! The size is frozen per world, like the generator version: it decides the
//! address, and the address is the save format.

use crate::BLOCK_M;
use crate::grid::Grid;

/// Smallest body: a sector side of exactly one chunk, so a chunk still tiles a
/// sector. Below this the block grid has no room for the unit of storage.
pub const MIN_BITS: u32 = 4;

/// The human half of the build band, in blocks: deep enough for a cellar, tall
/// enough for a tower. An absolute measure, because a cellar is the same depth
/// whatever the body is (DECISIONS 05).
const BAND_BLOCKS: u32 = 256;
/// The structural half: whatever the band asks for, the hollow core keeps this
/// much of the radius. On a small body the two meet and the radius wins.
const CORE_SHARE: f64 = 0.75;

/// Six square grids of blocks projected on a sphere, at one size.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct QuadSphere {
    blocks: Grid,
}

impl QuadSphere {
    /// A body of `2^bits` blocks per sector side, or `None` outside
    /// [`MIN_BITS`]`..=`[`MAX_BITS`].
    pub const fn new(bits: u32) -> Option<QuadSphere> {
        if bits < MIN_BITS {
            return None;
        }
        match Grid::new(bits) {
            None => None,
            Some(blocks) => Some(QuadSphere { blocks }),
        }
    }

    /// The grid a block lives on: where seams, neighbours and directions are
    /// asked.
    pub const fn blocks(self) -> Grid {
        self.blocks
    }

    /// Blocks per sector side, as a power of two.
    pub const fn bits(self) -> u32 {
        self.blocks.bits()
    }

    /// Radius of the datum sphere (`h = 0`), in blocks. Four sector sides make
    /// one great circle.
    pub fn radius_blocks(self) -> f64 {
        f64::from(self.blocks.side()) * 4.0 / core::f64::consts::TAU
    }

    /// Radius of the datum sphere, in metres.
    pub fn radius_m(self) -> f64 {
        self.radius_blocks() * BLOCK_M
    }

    /// Half height of the build band, in blocks: `h` runs `-band..=band`.
    ///
    /// The human measure, unless the body is too small to hold it, in which
    /// case the core keeps its share and the band is what is left.
    pub fn band_blocks(self) -> u16 {
        let room = libm::floor(self.radius_blocks() * (1.0 - CORE_SHARE)) as u32;
        room.min(BAND_BLOCKS) as u16
    }

    /// Half height of the build band, in metres.
    pub fn band_m(self) -> f64 {
        f64::from(self.band_blocks()) * BLOCK_M
    }
}
