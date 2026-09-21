//! Chunk formats and codecs: what a stored piece of world is.
//!
//! One blob serves disk, wire and the client's cache (ARCHITECTURE, Voxels),
//! so nothing here is decoded on the way in or encoded on the way out: a
//! server hands over the bytes it already has.
//!
//! This crate knows what a cell **holds**, never what it **means**. A material
//! is a byte whose meaning belongs to the generator version the world was made
//! with, exactly as `params` does. That is what keeps storage under the
//! generator in the dependency order instead of beside it.
//!
//! The common chunk is empty of information: sky, or solid rock well under the
//! ground. A whole slab of the build band is one or the other, so [`Chunk`]
//! makes that case structural rather than an optimisation, and such a chunk
//! costs four bytes stored and four bytes in memory.
//!
//! Determinism, as everywhere: two clients that generate the same chunk must
//! write the same bytes, so the rounding goes through `libm`.

mod codec;
mod nets;

pub use codec::ChunkError;
pub use nets::{Ground, Mesh, Vertex, mesh};

/// A chunk is `2^CHUNK_BITS` cells on a side.
pub const CHUNK_BITS: u32 = 4;
/// Cells along one edge of a chunk.
pub const CHUNK_SIDE: usize = 1 << CHUNK_BITS;
/// Cells in a chunk.
pub const CHUNK_CELLS: usize = CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE;

/// How many cells of signed distance the stored byte spans each way. Past it a
/// cell only says which side of the ground it is on, which is all a mesher
/// ever asks of a cell that far away.
pub const DENSITY_REACH: f64 = 2.0;

/// What covers a cell. The number belongs to a generator version; this crate
/// only carries it.
pub type Material = u8;

/// Signed distance from a cell's centre to the ground, in cells, positive
/// inside the ground. Quantised to a byte over [`DENSITY_REACH`] each way.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Density(i8);

impl Density {
    /// Ground so far above a cell that only the sign of it matters.
    pub const SOLID: Density = Density(i8::MAX);
    /// The same, with the ground far below.
    pub const AIR: Density = Density(i8::MIN);

    pub fn from_cells(cells: f64) -> Density {
        let scale = f64::from(i8::MAX) / DENSITY_REACH;
        let quantised = libm::round(cells * scale);
        Density(quantised.clamp(f64::from(i8::MIN), f64::from(i8::MAX)) as i8)
    }

    pub fn cells(self) -> f64 {
        f64::from(self.0) * DENSITY_REACH / f64::from(i8::MAX)
    }

    /// Whether the cell's centre is in the ground. The surface is the change.
    pub fn solid(self) -> bool {
        self.0 > 0
    }

    fn bits(self) -> i8 {
        self.0
    }

    fn from_bits(bits: i8) -> Density {
        Density(bits)
    }
}

/// A cell of the terrain layer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub density: Density,
    pub material: Material,
}

/// The cells of one chunk, in `u`, then `v`, then `h`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cells {
    density: [i8; CHUNK_CELLS],
    material: [Material; CHUNK_CELLS],
}

/// One chunk of the terrain layer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Chunk {
    /// Every cell alike: sky, or rock well under the ground. Most of a world.
    Uniform(Cell),
    /// Cell by cell. Only ground the surface or an edit runs through.
    Cells(Box<Cells>),
}

/// Index of a cell inside a chunk, from its `[u, v, h]` in `0..CHUNK_SIDE`.
pub fn index(at: [usize; 3]) -> usize {
    debug_assert!(at.iter().all(|k| *k < CHUNK_SIDE));
    at[0] + CHUNK_SIDE * (at[1] + CHUNK_SIDE * at[2])
}

impl Chunk {
    pub fn uniform(density: Density, material: Material) -> Chunk {
        Chunk::Uniform(Cell { density, material })
    }

    /// Fills a chunk cell by cell, and keeps it whole only if it says
    /// something: a chunk of one repeated cell collapses on the way in, so
    /// nothing downstream has to notice that it could have.
    pub fn from_fn(mut cell: impl FnMut([usize; 3]) -> Cell) -> Chunk {
        let mut cells = Cells {
            density: [0; CHUNK_CELLS],
            material: [0; CHUNK_CELLS],
        };
        let mut first = None;
        let mut varies = false;
        for h in 0..CHUNK_SIDE {
            for v in 0..CHUNK_SIDE {
                for u in 0..CHUNK_SIDE {
                    let at = [u, v, h];
                    let value = cell(at);
                    let i = index(at);
                    cells.density[i] = value.density.bits();
                    cells.material[i] = value.material;
                    match first {
                        None => first = Some(value),
                        Some(first) => varies |= first != value,
                    }
                }
            }
        }
        match (varies, first) {
            (false, Some(cell)) => Chunk::Uniform(cell),
            _ => Chunk::Cells(Box::new(cells)),
        }
    }

    pub fn cell(&self, at: [usize; 3]) -> Cell {
        match self {
            Chunk::Uniform(cell) => *cell,
            Chunk::Cells(cells) => {
                let i = index(at);
                Cell {
                    density: Density::from_bits(cells.density[i]),
                    material: cells.material[i],
                }
            }
        }
    }

    /// Whether the ground passes through this chunk at all. A mesher can stop
    /// here: a chunk with no change of sign has no surface in it.
    pub fn has_surface(&self) -> bool {
        match self {
            Chunk::Uniform(_) => false,
            Chunk::Cells(cells) => {
                let first = cells.density[0] > 0;
                cells.density.iter().any(|d| (*d > 0) != first)
            }
        }
    }
}
