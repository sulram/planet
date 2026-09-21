//! The read path, and the lattice a mesher reads through it.
//!
//! `chunk(addr)` is the stored chunk if there is one and the generated chunk
//! otherwise, and a caller never learns which. Nothing is stored yet, so today
//! every answer is generated once and kept.

use glam::DVec3;
use std::collections::HashMap;

use topology::{Column, QuadSphere};
use voxel::{CHUNK_BITS, CHUNK_SIDE, Chunk, Material};
use worldgen::Generator;

use crate::address::{ChunkAddr, cell_grid};
use crate::generate::{cell_point, generate};

/// What one call to [`Chunks::warm`] did.
pub struct Warmth {
    /// Chunks generated, which is what a frame's budget is spent on.
    pub made: usize,
    /// Whether everything a mesh will read is now held.
    pub whole: bool,
}

/// Every chunk this client is holding, by address.
///
/// A chunk is kept with where it is, because working that out means folding a
/// point over a seam and a tangent: cheap once, ruinous once per comparison of
/// a sort (measured: it was the whole frame).
#[derive(Default)]
pub struct Chunks {
    held: HashMap<ChunkAddr, Chunk>,
    centres: HashMap<ChunkAddr, DVec3>,
}

impl Chunks {
    pub fn new() -> Chunks {
        Chunks::default()
    }

    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// How many held chunks say one thing: all rock, or all sky. A hierarchy
    /// would carry each of these as one value on a parent instead of as a
    /// chunk of its own (VDB's tiles, refs/dust).
    pub fn uniform(&self) -> usize {
        self.held
            .values()
            .filter(|chunk| matches!(chunk, Chunk::Uniform(_)))
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// The chunk at an address, generating it if it is not held. The one read
    /// path: a caller never knows whether the world was stored or made.
    pub fn chunk(&mut self, generator: &Generator, sphere: QuadSphere, addr: ChunkAddr) -> &Chunk {
        self.centres
            .entry(addr)
            .or_insert_with(|| crate::stream::chunk_centre(sphere, addr));
        self.held
            .entry(addr)
            .or_insert_with(|| generate(generator, sphere, addr))
    }

    /// Where a held chunk sits, metres from the body's centre.
    pub fn centre(&self, addr: &ChunkAddr) -> Option<DVec3> {
        self.centres.get(addr).copied()
    }

    /// Brings in everything [`Lattice`] will ask for around one chunk: itself
    /// and the 26 chunks it touches, because a mesh reads one cell past its
    /// own on every side.
    ///
    /// Generates at most `limit` of them and says whether the neighbourhood is
    /// now whole. A frame must be able to stop in the middle: a fresh chunk
    /// wants 27 of these, and a budget that cannot cut inside one of them is
    /// not a budget (measured: one chunk cost a whole frame and a half).
    pub fn warm(
        &mut self,
        generator: &Generator,
        sphere: QuadSphere,
        addr: ChunkAddr,
        limit: usize,
    ) -> Warmth {
        let mut made = 0;
        let mut whole = true;
        for offset in NEIGHBOURHOOD {
            let Some(at) = self.neighbour(sphere, addr, offset) else {
                continue;
            };
            if self.held.contains_key(&at) {
                continue;
            }
            if made == limit {
                whole = false;
                break;
            }
            self.chunk(generator, sphere, at);
            made += 1;
        }
        Warmth { made, whole }
    }

    pub fn forget(&mut self, addr: &ChunkAddr) {
        self.held.remove(addr);
        self.centres.remove(addr);
    }

    /// Drops every chunk further from `eye` than its own level allows.
    pub fn retain_near(&mut self, eye: DVec3, reach_m: impl Fn(&ChunkAddr) -> f64) {
        let centres = &mut self.centres;
        self.held.retain(|addr, _| match centres.get(addr) {
            Some(centre) => centre.distance(eye) <= reach_m(addr),
            None => false,
        });
        centres.retain(|addr, _| self.held.contains_key(addr));
    }

    /// The chunk one step away, over a seam if that is where it is. Always of
    /// the same level: a mesh reads its own grain and no other.
    fn neighbour(&self, sphere: QuadSphere, addr: ChunkAddr, offset: [i32; 3]) -> Option<ChunkAddr> {
        let (column, h) = locate(sphere, addr, [
            offset[0] * CHUNK_SIDE as i32,
            offset[1] * CHUNK_SIDE as i32,
            offset[2] * CHUNK_SIDE as i32,
        ])?;
        Some(chunk_of(addr.level, column, h))
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
    let cells = cell_grid(sphere, addr.level)?;
    let h = addr.low_h().checked_add(offset[2])?;
    let low = addr.low_column();
    let side = cells.side() as i32;
    let (u, v) = (
        i32::from(low.u) + offset[0],
        i32::from(low.v) + offset[1],
    );
    // Still inside the sector, which nearly every border cell is: plain
    // integers, and no fold to pay for.
    if (0..side).contains(&u) && (0..side).contains(&v) {
        return Some((Column::new(low.sector, u as u16, v as u16), h));
    }
    let point = cells.wrapped(cell_point(addr, offset[0], offset[1]));
    Some((cells.column_of(point), h))
}

/// The chunk of `level` that holds a cell, and where in it.
pub fn chunk_of(level: crate::address::Level, column: Column, h: i32) -> ChunkAddr {
    ChunkAddr::new(
        level,
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
        if at.iter().all(|k| (0..side).contains(k)) {
            if let Some(chunk) = self.chunks.held.get(&self.addr) {
                return chunk.cell([at[0] as usize, at[1] as usize, at[2] as usize]);
            }
            return self.sky();
        }
        // Outside: find the cell once, then the chunk that holds it.
        let Some((column, h)) = locate(self.sphere, self.addr, at) else {
            return self.sky();
        };
        match self.chunks.held.get(&chunk_of(self.addr.level, column, h)) {
            Some(chunk) => chunk.cell(cell_of(column, h)),
            None => self.sky(),
        }
    }

    /// What a cell nobody holds reads as.
    fn sky(&self) -> voxel::Cell {
        voxel::Cell {
            density: voxel::Density::from_cells(f64::from(self.outside)),
            material: 0,
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
