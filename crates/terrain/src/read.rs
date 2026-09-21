//! The read path, and the lattice a mesher reads through it.
//!
//! `chunk(addr)` is the stored chunk if there is one and the generated chunk
//! otherwise, and a caller never learns which. Nothing is stored yet, so today
//! every answer is generated once and kept.

use std::collections::HashMap;

use topology::{Column, Grid, QuadSphere};
use voxel::{CHUNK_BITS, CHUNK_SIDE, Chunk, Material};
use worldgen::Generator;

use crate::address::ChunkAddr;
use crate::generate::{cell_point, generate};

/// Every chunk this client is holding, by address.
#[derive(Default)]
pub struct Chunks {
    held: HashMap<ChunkAddr, Chunk>,
}

impl Chunks {
    pub fn new() -> Chunks {
        Chunks::default()
    }

    pub fn len(&self) -> usize {
        self.held.len()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// The chunk at an address, generating it if it is not held. The one read
    /// path: a caller never knows whether the world was stored or made.
    pub fn chunk(&mut self, generator: &Generator, sphere: QuadSphere, addr: ChunkAddr) -> &Chunk {
        self.held
            .entry(addr)
            .or_insert_with(|| generate(generator, sphere, addr))
    }

    /// Brings in everything [`Lattice`] will ask for around one chunk: itself
    /// and the 26 chunks it touches, because a mesh reads one cell past its
    /// own on every side.
    pub fn warm(&mut self, generator: &Generator, sphere: QuadSphere, addr: ChunkAddr) {
        for offset in NEIGHBOURHOOD {
            if let Some(at) = self.neighbour(sphere, addr, offset) {
                self.chunk(generator, sphere, at);
            }
        }
    }

    pub fn forget(&mut self, addr: &ChunkAddr) {
        self.held.remove(addr);
    }

    /// The chunk one step away, over a seam if that is where it is.
    fn neighbour(&self, sphere: QuadSphere, addr: ChunkAddr, offset: [i32; 3]) -> Option<ChunkAddr> {
        let (column, h) = locate(sphere, addr, [
            offset[0] * CHUNK_SIDE as i32,
            offset[1] * CHUNK_SIDE as i32,
            offset[2] * CHUNK_SIDE as i32,
        ])?;
        Some(chunk_of(column, h))
    }
}

/// The 27 chunks a mesh reads: itself and everything it touches.
const NEIGHBOURHOOD: [[i32; 3]; 27] = {
    let mut all = [[0; 3]; 27];
    let mut i = 0;
    let mut z = -1;
    while z <= 1 {
        let mut y = -1;
        while y <= 1 {
            let mut x = -1;
            while x <= 1 {
                all[i] = [x, y, z];
                i += 1;
                x += 1;
            }
            y += 1;
        }
        z += 1;
    }
    all
};

/// The block a lattice point `offset` cells from a chunk's low corner lands
/// on, folded over a seam when it leaves the sector. `None` above or below
/// what an `i16` height can name.
pub fn locate(sphere: QuadSphere, addr: ChunkAddr, offset: [i32; 3]) -> Option<(Column, i32)> {
    let blocks = sphere.blocks();
    let point = blocks.wrapped(cell_point(addr, offset[0], offset[1]));
    let h = addr.low_h().checked_add(offset[2])?;
    Some((blocks.column_of(point), h))
}

/// The chunk that holds a block, and where in it.
pub fn chunk_of(column: Column, h: i32) -> ChunkAddr {
    ChunkAddr::new(
        Column::new(
            column.sector,
            column.u >> CHUNK_BITS,
            column.v >> CHUNK_BITS,
        ),
        h.div_euclid(CHUNK_SIDE as i32) as i16,
    )
}

/// Where in its chunk a block sits.
pub fn cell_of(column: Column, h: i32) -> [usize; 3] {
    let mask = (CHUNK_SIDE - 1) as u16;
    [
        usize::from(column.u & mask),
        usize::from(column.v & mask),
        h.rem_euclid(CHUNK_SIDE as i32) as usize,
    ]
}

/// The sample lattice of one chunk, read through [`Chunks::chunk`].
///
/// The mesher asks for one cell past the chunk on every side. Those cells
/// belong to neighbours, and over a seam to a neighbour whose grid axes are
/// turned; the fold happens in block space, where it is property tested, so
/// nothing here knows what a seam is.
pub struct Lattice<'a> {
    pub chunks: &'a Chunks,
    pub sphere: QuadSphere,
    pub addr: ChunkAddr,
    /// Density of a cell nothing holds: sky.
    pub outside: f32,
}

impl Lattice<'_> {
    fn cell(&self, at: [i32; 3]) -> voxel::Cell {
        let side = CHUNK_SIDE as i32;
        let inside = at.iter().all(|k| (0..side).contains(k));
        let held = if inside {
            self.chunks.held.get(&self.addr)
        } else {
            match locate(self.sphere, self.addr, at) {
                None => None,
                Some((column, h)) => self.chunks.held.get(&chunk_of(column, h)),
            }
        };
        match held {
            Some(chunk) if inside => chunk.cell([at[0] as usize, at[1] as usize, at[2] as usize]),
            Some(chunk) => {
                let (column, h) = locate(self.sphere, self.addr, at).expect("a located cell");
                chunk.cell(cell_of(column, h))
            }
            None => voxel::Cell {
                density: voxel::Density::from_cells(f64::from(self.outside)),
                material: 0,
            },
        }
    }
}

impl voxel::Ground for Lattice<'_> {
    fn density(&self, at: [i32; 3]) -> f32 {
        self.cell(at).density.cells() as f32
    }

    fn material(&self, at: [i32; 3]) -> Material {
        self.cell(at).material
    }
}

/// The grid chunks are addressed on.
pub fn grid(sphere: QuadSphere) -> Grid {
    crate::address::chunk_grid(sphere)
}
