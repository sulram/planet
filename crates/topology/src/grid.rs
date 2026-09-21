//! A square grid on the six faces, and everything that does not need metres.
//!
//! `2^bits` cells a side. Nothing here knows whether a cell is a block, a
//! chunk or a cell of a reduced level: seams, neighbours and the warp are the
//! same arithmetic at every grain, which is why a chunk grid gets its seams
//! from the same property tests the block grid is held to.
//!
//! Metres belong to a [`QuadSphere`](crate::QuadSphere), which is one body's
//! block grid plus a radius.

/// The finest grid there is: `u` and `v` fill a `u16`.
pub const MAX_BITS: u32 = 16;

/// Six square grids on a cube, at one grain.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Grid {
    bits: u32,
}

impl Grid {
    /// A grid of `2^bits` cells per sector side, or `None` past [`MAX_BITS`].
    pub const fn new(bits: u32) -> Option<Grid> {
        if bits > MAX_BITS {
            return None;
        }
        Some(Grid { bits })
    }

    /// Cells per sector side, as a power of two.
    pub const fn bits(self) -> u32 {
        self.bits
    }

    /// Cells per sector side.
    pub const fn side(self) -> u32 {
        1 << self.bits
    }

    /// Highest legal `u` or `v`.
    pub const fn max_coord(self) -> u16 {
        (self.side() - 1) as u16
    }

    /// The same six faces in cells `2^levels` times as wide: the grid a chunk,
    /// or a reduced level, is addressed on. `None` when a cell would be wider
    /// than a sector.
    pub const fn coarsened(self, levels: u32) -> Option<Grid> {
        if levels > self.bits {
            return None;
        }
        Some(Grid {
            bits: self.bits - levels,
        })
    }

    /// Half a sector side, in cells: the centre of a face.
    pub(crate) fn half(self) -> f64 {
        f64::from(self.side()) / 2.0
    }
}
