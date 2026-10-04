//! A volume as it is seen from afar: a cell for every [`AFAR`] cells of it
//! each way, solid where any of those is, in the paint most of them have.
//! What a world sends of a volume too far to hold whole, and what a client
//! draws of it: a few cubes of two metres where there were thousands.
//!
//! Any solid cell makes the cell afar solid, so a wall one cell thick, a
//! pillar or a lamp still stands out on the skyline: thicker, never gone.

use crate::{CHUNK, Cell};

/// Cells of a volume along each side of one of its cells seen from afar.
pub const AFAR: i32 = 4;
/// Cells afar along the side of a chunk.
pub const CHUNK_AFAR: i32 = CHUNK as i32 / AFAR;
/// Cells afar in a chunk.
pub const CHUNK_AFAR_CELLS: usize = (CHUNK_AFAR * CHUNK_AFAR * CHUNK_AFAR) as usize;

/// The cells of a chunk, `x` fastest, then `y`, then `z`, as they are seen
/// from afar, in the same order.
pub fn afar(chunk: &[Cell]) -> Vec<Cell> {
    let side = CHUNK as usize;
    let block = AFAR as usize;
    let mut out = Vec::with_capacity(CHUNK_AFAR_CELLS);
    let mut counts: Vec<(Cell, u8)> = Vec::with_capacity(block * block * block);
    for z in 0..CHUNK_AFAR as usize {
        for y in 0..CHUNK_AFAR as usize {
            for x in 0..CHUNK_AFAR as usize {
                counts.clear();
                for dz in 0..block {
                    for dy in 0..block {
                        let row = ((z * block + dz) * side + y * block + dy) * side + x * block;
                        for &cell in &chunk[row..row + block] {
                            if cell.is_air() {
                                continue;
                            }
                            match counts.iter_mut().find(|(seen, _)| *seen == cell) {
                                Some((_, count)) => *count += 1,
                                None => counts.push((cell, 1)),
                            }
                        }
                    }
                }
                // The most common paint; between two as common, the first met.
                let most =
                    counts.iter().fold(
                        None,
                        |best: Option<(Cell, u8)>, &(cell, count)| match best {
                            Some((_, most)) if most >= count => best,
                            _ => Some((cell, count)),
                        },
                    );
                out.push(most.map_or(Cell::AIR, |(cell, _)| cell));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(solid: impl Fn([usize; 3]) -> Option<u8>) -> Vec<Cell> {
        let side = CHUNK as usize;
        (0..side * side * side)
            .map(|at| {
                let p = [at % side, at / side % side, at / (side * side)];
                solid(p).map_or(Cell::AIR, Cell::solid)
            })
            .collect()
    }

    #[test]
    fn a_wall_one_cell_thick_stands_afar_and_air_stays_air() {
        // A wall along x at y = 5, in paint 3 with a stripe of paint 7.
        let cells = chunk(|[_, y, z]| (y == 5).then_some(if z == 2 { 7 } else { 3 }));
        let seen = afar(&cells);
        assert_eq!(seen.len(), CHUNK_AFAR_CELLS);
        let at = |x: usize, y: usize, z: usize| seen[(z * 4 + y) * 4 + x];
        for x in 0..4 {
            for z in 0..4 {
                assert_eq!(at(x, 1, z), Cell::solid(3), "the wall at {x} {z}");
                assert!(at(x, 0, z).is_air() && at(x, 2, z).is_air());
            }
        }
        assert!(afar(&chunk(|_| None)).iter().all(|cell| cell.is_air()));
    }

    #[test]
    fn a_lone_cell_is_seen_afar() {
        let cells = chunk(|p| (p == [13, 2, 9]).then_some(15));
        let seen = afar(&cells);
        let solid: Vec<usize> = (0..seen.len()).filter(|&at| !seen[at].is_air()).collect();
        assert_eq!(solid, vec![(2 * 4) * 4 + 3]);
        assert_eq!(seen[solid[0]], Cell::solid(15));
    }
}
