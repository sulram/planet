//! The chunk blob: the one encoding disk, wire and cache all share.
//!
//! Compression is deliberately not here yet. The format carries its version,
//! so wrapping the body in zstd is a second kind that costs nothing to add
//! when the wire path needs it (ARCHITECTURE, Voxels). Adding the dependency
//! before anything sends a chunk would be a guess about what it costs.

use crate::{CHUNK_CELLS, Cell, Cells, Chunk, Density, Material};

/// The only format this build writes.
const VERSION: u8 = 1;
const UNIFORM: u8 = 0;
const CELLS: u8 = 1;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ChunkError {
    /// A blob from a newer build, or not a blob at all.
    UnknownVersion(u8),
    UnknownKind(u8),
    /// The header promises more than the bytes hold.
    Truncated,
    /// A palette must name at least one material and no more than 256.
    BadPalette(usize),
}

impl core::fmt::Display for ChunkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChunkError::UnknownVersion(v) => write!(f, "unknown chunk format {v}"),
            ChunkError::UnknownKind(k) => write!(f, "unknown chunk kind {k}"),
            ChunkError::Truncated => write!(f, "chunk is shorter than its header promises"),
            ChunkError::BadPalette(n) => write!(f, "a palette of {n} materials"),
        }
    }
}

impl std::error::Error for ChunkError {}

/// Bits enough to tell `n` materials apart. One material needs none: the
/// indices are then absent from the blob entirely.
fn index_bits(n: usize) -> u32 {
    match n {
        0 | 1 => 0,
        n => (usize::BITS - (n - 1).leading_zeros()).max(1),
    }
}

impl Chunk {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![VERSION];
        match self {
            Chunk::Uniform(cell) => {
                out.push(UNIFORM);
                out.push(cell.density.bits() as u8);
                out.push(cell.material);
            }
            Chunk::Cells(cells) => {
                out.push(CELLS);
                // The palette is the materials this chunk actually uses, in
                // the order it meets them: a chunk of rock and air spends one
                // bit a cell instead of eight.
                let mut palette: Vec<Material> = Vec::new();
                let mut indices = Vec::with_capacity(CHUNK_CELLS);
                for material in cells.material {
                    let at = palette
                        .iter()
                        .position(|m| *m == material)
                        .unwrap_or_else(|| {
                            palette.push(material);
                            palette.len() - 1
                        });
                    indices.push(at);
                }
                out.push((palette.len() - 1) as u8);
                out.extend_from_slice(&palette);
                out.extend(cells.density.iter().map(|d| *d as u8));
                out.extend(pack(&indices, index_bits(palette.len())));
            }
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Chunk, ChunkError> {
        let &[version, kind, ref body @ ..] = bytes else {
            return Err(ChunkError::Truncated);
        };
        if version != VERSION {
            return Err(ChunkError::UnknownVersion(version));
        }
        match kind {
            UNIFORM => {
                let &[density, material, ..] = body else {
                    return Err(ChunkError::Truncated);
                };
                Ok(Chunk::Uniform(Cell {
                    density: Density::from_bits(density as i8),
                    material,
                }))
            }
            CELLS => {
                let &[last, ref body @ ..] = body else {
                    return Err(ChunkError::Truncated);
                };
                let count = usize::from(last) + 1;
                if body.len() < count {
                    return Err(ChunkError::Truncated);
                }
                let (palette, body) = body.split_at(count);
                if body.len() < CHUNK_CELLS {
                    return Err(ChunkError::Truncated);
                }
                let (density, body) = body.split_at(CHUNK_CELLS);
                let bits = index_bits(count);
                let indices = unpack(body, bits, CHUNK_CELLS).ok_or(ChunkError::Truncated)?;
                let mut cells = Cells {
                    density: [0; CHUNK_CELLS],
                    material: [0; CHUNK_CELLS],
                };
                for (i, byte) in density.iter().enumerate() {
                    cells.density[i] = *byte as i8;
                }
                for (i, at) in indices.iter().enumerate() {
                    cells.material[i] = *palette.get(*at).ok_or(ChunkError::BadPalette(count))?;
                }
                Ok(Chunk::Cells(Box::new(cells)))
            }
            kind => Err(ChunkError::UnknownKind(kind)),
        }
    }
}

/// Little endian bit packing, lowest bit of the first index first.
fn pack(indices: &[usize], bits: u32) -> Vec<u8> {
    if bits == 0 {
        return Vec::new();
    }
    let mut out = vec![0u8; (indices.len() * bits as usize).div_ceil(8)];
    for (i, index) in indices.iter().enumerate() {
        let start = i * bits as usize;
        for bit in 0..bits as usize {
            if index >> bit & 1 == 1 {
                let at = start + bit;
                out[at / 8] |= 1 << (at % 8);
            }
        }
    }
    out
}

fn unpack(bytes: &[u8], bits: u32, count: usize) -> Option<Vec<usize>> {
    if bits == 0 {
        return Some(vec![0; count]);
    }
    if bytes.len() < (count * bits as usize).div_ceil(8) {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * bits as usize;
        let mut index = 0usize;
        for bit in 0..bits as usize {
            let at = start + bit;
            if bytes[at / 8] >> (at % 8) & 1 == 1 {
                index |= 1 << bit;
            }
        }
        out.push(index);
    }
    Some(out)
}
