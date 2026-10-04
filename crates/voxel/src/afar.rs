//! A volume as it is seen from afar: a cell for every [`AFAR`] cells of it
//! each way, solid where any of those is, in the paint most of them have.
//! What a world sends of a volume too far to hold whole, and what a client
//! draws of it: a few cubes of two metres where there were thousands. Seen
//! from further still, the same again of those: a cube of eight metres for
//! every chunk.
//!
//! Any solid cell makes the cell afar solid, so a wall one cell thick, a
//! pillar or a lamp still stands out on the skyline: thicker, never gone.

use crate::{CHUNK, Cell};

/// Cells along each side of a cell seen from afar, of the cells one step
/// nearer.
pub const AFAR: i32 = 4;

/// A cube of cells `side` along each edge, `x` fastest, then `y`, then `z`,
/// as it is seen from one step further off: `side / AFAR` along each edge,
/// in the same order.
pub fn afar(cells: &[Cell], side: usize) -> Vec<Cell> {
    let block = AFAR as usize;
    let out_side = side / block;
    let mut out = Vec::with_capacity(out_side * out_side * out_side);
    let mut counts: Vec<(Cell, u8)> = Vec::with_capacity(block * block * block);
    for z in 0..out_side {
        for y in 0..out_side {
            for x in 0..out_side {
                counts.clear();
                for dz in 0..block {
                    for dy in 0..block {
                        let row = ((z * block + dz) * side + y * block + dy) * side + x * block;
                        for &cell in &cells[row..row + block] {
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

/// The cells of a chunk as they are seen from `scale` cells each way, a
/// power of [`AFAR`] no wider than the chunk: the chunk itself at 1, a cell
/// for the chunk at its side.
pub fn chunk_afar(chunk: &[Cell], scale: i32) -> Vec<Cell> {
    let (mut cells, mut side, mut at) = (chunk.to_vec(), CHUNK as usize, 1);
    while at < scale && side >= AFAR as usize {
        cells = afar(&cells, side);
        side /= AFAR as usize;
        at *= AFAR;
    }
    cells
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
        let seen = afar(&cells, CHUNK as usize);
        assert_eq!(seen.len(), 64);
        let at = |x: usize, y: usize, z: usize| seen[(z * 4 + y) * 4 + x];
        for x in 0..4 {
            for z in 0..4 {
                assert_eq!(at(x, 1, z), Cell::solid(3), "the wall at {x} {z}");
                assert!(at(x, 0, z).is_air() && at(x, 2, z).is_air());
            }
        }
        assert!(afar(&chunk(|_| None), 16).iter().all(|cell| cell.is_air()));
    }

    #[test]
    fn a_lone_cell_is_seen_afar_and_further_still() {
        let cells = chunk(|p| (p == [13, 2, 9]).then_some(15));
        let seen = chunk_afar(&cells, AFAR);
        let solid: Vec<usize> = (0..seen.len()).filter(|&at| !seen[at].is_air()).collect();
        assert_eq!(solid, vec![(2 * 4) * 4 + 3]);
        assert_eq!(seen[solid[0]], Cell::solid(15));
        // From a step further, the chunk is one cell, in that paint.
        assert_eq!(chunk_afar(&cells, AFAR * AFAR), vec![Cell::solid(15)]);
        assert_eq!(chunk_afar(&cells, 1), cells);
    }
}
