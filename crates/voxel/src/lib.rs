//! The build layer: cubes in an integer box.
//!
//! A volume knows no sphere (CLAUDE.md, How it grows). Its cells are unit
//! cubes in a frame of its own, `x` and `y` across and `z` up, and seating it
//! on a body is the client's job. It knows no ground either: what is not a
//! cell is air, and whoever seats a volume says where the ground passes.
//!
//! Volumes stand side by side on a fixed grid of plots ([`Volumes`]) and are
//! read as one, so what is too big for one volume stands over two
//! neighbours.
//!
//! What lives here is what every front end and the server will agree on: what
//! a gesture does to the cells ([`Gesture`]), which cell a line of sight meets
//! ([`Volumes::trace`]), which faces show and how shut in their corners are
//! ([`Volumes::faces`]), and what a body standing in a column has under and
//! over it ([`Volumes::gap`]).

mod faces;
mod gesture;
mod trace;
mod volumes;

pub use faces::Quad;
pub use gesture::Gesture;
pub use trace::{Hit, crossing};
pub use volumes::Volumes;

/// Cells per chunk side, as a power of two.
pub const CHUNK_BITS: u32 = 4;
/// Cells per chunk side: the unit a volume is stored, meshed and redrawn in.
pub const CHUNK: u32 = 1 << CHUNK_BITS;
const CHUNK_CELLS: usize = (CHUNK * CHUNK * CHUNK) as usize;

/// What fills one cell: air, or a paint, which is an index into whatever
/// palette the volume is shown with.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Cell(u8);

impl Cell {
    pub const AIR: Cell = Cell(0);

    /// The most paints a cell can name.
    pub const PAINTS: usize = 255;

    /// A solid cell of one paint.
    pub const fn solid(paint: u8) -> Cell {
        // Paint 255 would wrap to air; it is folded onto the last one.
        Cell(if paint == u8::MAX { u8::MAX } else { paint + 1 })
    }

    pub const fn is_air(self) -> bool {
        self.0 == 0
    }

    /// The paint of a solid cell, `None` for air.
    pub const fn paint(self) -> Option<u8> {
        match self.0 {
            0 => None,
            n => Some(n - 1),
        }
    }
}

/// Cells as bytes, for the wire and for what a world keeps: runs of one cell,
/// each a count and the cell. A chunk of air and a slab is a few bytes.
pub fn pack(cells: &[Cell]) -> Vec<u8> {
    let mut packed = Vec::new();
    let mut at = 0;
    while at < cells.len() {
        let cell = cells[at];
        let run = cells[at..]
            .iter()
            .take(usize::from(u8::MAX))
            .take_while(|&&next| next == cell)
            .count();
        packed.extend([run as u8, cell.0]);
        at += run;
    }
    packed
}

/// The cells [`pack`] packed, when they are exactly `count` of them: `None`
/// for bytes that are no such thing.
pub fn unpack(packed: &[u8], count: usize) -> Option<Vec<Cell>> {
    let mut cells = Vec::with_capacity(count);
    for run in packed.chunks(2) {
        let &[run, cell] = run else {
            return None;
        };
        if run == 0 || cells.len() + usize::from(run) > count {
            return None;
        }
        cells.extend(core::iter::repeat_n(Cell(cell), usize::from(run)));
    }
    (cells.len() == count).then_some(cells)
}

/// One side of a cell: an axis and which way along it the side faces.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Face {
    /// `0` for `x`, `1` for `y`, `2` for `z`.
    pub axis: usize,
    /// Facing up the axis.
    pub positive: bool,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face::new(0, true),
        Face::new(0, false),
        Face::new(1, true),
        Face::new(1, false),
        Face::new(2, true),
        Face::new(2, false),
    ];

    /// The top of a cell.
    pub const UP: Face = Face::new(2, true);

    pub const fn new(axis: usize, positive: bool) -> Face {
        Face { axis, positive }
    }

    /// The step from a cell to the one this side looks at.
    pub fn normal(self) -> [i32; 3] {
        let mut n = [0; 3];
        n[self.axis] = if self.positive { 1 } else { -1 };
        n
    }
}

/// A box of cells, both corners included.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

impl Span {
    /// The box a drag from one cell to another sweeps, whichever way it went.
    pub fn between(a: [i32; 3], b: [i32; 3]) -> Span {
        Span {
            min: [0, 1, 2].map(|i| a[i].min(b[i])),
            max: [0, 1, 2].map(|i| a[i].max(b[i])),
        }
    }

    pub fn cell(at: [i32; 3]) -> Span {
        Span { min: at, max: at }
    }

    /// Cells along each axis.
    pub fn size(self) -> [u32; 3] {
        [0, 1, 2].map(|i| (self.max[i] - self.min[i] + 1) as u32)
    }

    pub fn contains(self, at: [i32; 3]) -> bool {
        (0..3).all(|i| self.min[i] <= at[i] && at[i] <= self.max[i])
    }

    /// Grown just enough to hold a cell.
    pub fn with(self, at: [i32; 3]) -> Span {
        Span {
            min: [0, 1, 2].map(|i| self.min[i].min(at[i])),
            max: [0, 1, 2].map(|i| self.max[i].max(at[i])),
        }
    }

    /// Larger by `by` cells on every side.
    pub fn grown(self, by: i32) -> Span {
        Span {
            min: self.min.map(|n| n - by),
            max: self.max.map(|n| n + by),
        }
    }

    /// The same box `by` cells away.
    pub fn moved(self, by: [i32; 3]) -> Span {
        Span {
            min: [0, 1, 2].map(|i| self.min[i] + by[i]),
            max: [0, 1, 2].map(|i| self.max[i] + by[i]),
        }
    }

    /// What this box and another have in common.
    pub fn meet(self, other: Span) -> Option<Span> {
        let min = [0, 1, 2].map(|i| self.min[i].max(other.min[i]));
        let max = [0, 1, 2].map(|i| self.max[i].min(other.max[i]));
        (0..3)
            .all(|i| min[i] <= max[i])
            .then_some(Span { min, max })
    }

    /// Every cell, `x` fastest.
    pub fn cells(self) -> impl Iterator<Item = [i32; 3]> {
        let Span { min, max } = self;
        (min[2]..=max[2]).flat_map(move |z| {
            (min[1]..=max[1]).flat_map(move |y| (min[0]..=max[0]).map(move |x| [x, y, z]))
        })
    }

    /// Where a cell of the box comes in [`Span::cells`] order.
    pub(crate) fn index(self, at: [i32; 3]) -> usize {
        let [sx, sy, _] = self.size().map(|n| n as usize);
        let [x, y, z] = [0, 1, 2].map(|i| (at[i] - self.min[i]) as usize);
        (z * sy + y) * sx + x
    }
}

/// What holds up a body standing in one column: the top of the solid under
/// its feet and the bottom of the solid over its head, in cells.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gap {
    /// `None` when nothing is under the feet down to the bottom of the
    /// volume.
    pub floor: Option<i32>,
    /// `None` when nothing is over the head up to the top of the volume.
    pub ceiling: Option<i32>,
}

/// What the sides that show and a line of sight read of cells, whether one
/// volume holds them or several side by side.
pub(crate) trait Cells {
    /// The cell at a place, air where there is none.
    fn get(&self, at: [i32; 3]) -> Cell;

    /// Whether a place is solid to a ray, a body or the light.
    fn solid(&self, at: [i32; 3]) -> bool {
        !self.get(at).is_air()
    }
}

impl Cells for Volume {
    fn get(&self, at: [i32; 3]) -> Cell {
        Volume::get(self, at)
    }
}

/// Cells of a chunk, and how many of them are solid, so an emptied chunk is
/// dropped rather than kept as a block of air.
#[derive(Clone)]
struct Chunk {
    cells: Box<[Cell; CHUNK_CELLS]>,
    solid: u32,
}

/// An integer box of cells, stored by chunk. A chunk with nothing in it takes
/// no room.
#[derive(Clone)]
pub struct Volume {
    size: [u32; 3],
    chunks: Vec<Option<Chunk>>,
}

impl core::fmt::Debug for Volume {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let stored = self.chunks.iter().flatten().count();
        f.debug_struct("Volume")
            .field("size", &self.size)
            .field("stored_chunks", &stored)
            .finish()
    }
}

impl Volume {
    /// An empty box of `size` cells. Every side is at least one cell.
    pub fn new(size: [u32; 3]) -> Volume {
        let size = size.map(|n| n.max(1));
        let count = Volume::chunks_along(size).iter().product::<u32>() as usize;
        Volume {
            size,
            chunks: vec![None; count],
        }
    }

    pub fn size(&self) -> [u32; 3] {
        self.size
    }

    /// Every cell of the box.
    pub fn bounds(&self) -> Span {
        Span {
            min: [0; 3],
            max: self.size.map(|n| n as i32 - 1),
        }
    }

    /// Chunks along each axis.
    pub fn chunk_count(&self) -> [u32; 3] {
        Volume::chunks_along(self.size)
    }

    fn chunks_along(size: [u32; 3]) -> [u32; 3] {
        size.map(|n| n.div_ceil(CHUNK))
    }

    pub fn contains(&self, at: [i32; 3]) -> bool {
        self.bounds().contains(at)
    }

    /// Whether a column passes through this box.
    pub fn over(&self, x: i32, y: i32) -> bool {
        (0..self.size[0] as i32).contains(&x) && (0..self.size[1] as i32).contains(&y)
    }

    /// The cell at a place, air outside the box.
    pub fn get(&self, at: [i32; 3]) -> Cell {
        if !self.contains(at) {
            return Cell::AIR;
        }
        let (chunk, index) = self.locate(at);
        self.chunks[chunk]
            .as_ref()
            .map_or(Cell::AIR, |chunk| chunk.cells[index])
    }

    /// Whether a place is solid to a ray, a body or the light: a cell that is
    /// not air.
    pub fn solid(&self, at: [i32; 3]) -> bool {
        !self.get(at).is_air()
    }

    /// Writes one cell; a place outside the box is ignored. True when the cell
    /// changed.
    pub(crate) fn set(&mut self, at: [i32; 3], cell: Cell) -> bool {
        if !self.contains(at) {
            return false;
        }
        let (chunk, index) = self.locate(at);
        let slot = &mut self.chunks[chunk];
        let old = slot.as_ref().map_or(Cell::AIR, |chunk| chunk.cells[index]);
        if old == cell {
            return false;
        }
        let chunk = slot.get_or_insert_with(|| Chunk {
            cells: Box::new([Cell::AIR; CHUNK_CELLS]),
            solid: 0,
        });
        chunk.cells[index] = cell;
        match (old.is_air(), cell.is_air()) {
            (true, false) => chunk.solid += 1,
            (false, true) => chunk.solid -= 1,
            _ => {}
        }
        if chunk.solid == 0 {
            *slot = None;
        }
        true
    }

    /// Whether a chunk holds anything at all. `false` outside the box.
    pub fn chunk_stored(&self, chunk: [u32; 3]) -> bool {
        let [cx, cy, cz] = self.chunk_count();
        if chunk[0] >= cx || chunk[1] >= cy || chunk[2] >= cz {
            return false;
        }
        let index = (chunk[2] * cy + chunk[1]) * cx + chunk[0];
        self.chunks[index as usize].is_some()
    }

    /// The cells of one chunk, clipped to the box.
    pub fn chunk_span(&self, chunk: [u32; 3]) -> Span {
        let min = chunk.map(|c| (c * CHUNK) as i32);
        let span = Span {
            min,
            max: min.map(|n| n + CHUNK as i32 - 1),
        };
        span.meet(self.bounds()).unwrap_or(span)
    }

    /// Every chunk a box of cells reaches into.
    pub fn chunks_in(&self, span: Span) -> Vec<[u32; 3]> {
        let Some(span) = span.meet(self.bounds()) else {
            return Vec::new();
        };
        let lo = span.min.map(|n| (n as u32) >> CHUNK_BITS);
        let hi = span.max.map(|n| (n as u32) >> CHUNK_BITS);
        let mut out = Vec::new();
        for z in lo[2]..=hi[2] {
            for y in lo[1]..=hi[1] {
                for x in lo[0]..=hi[0] {
                    out.push([x, y, z]);
                }
            }
        }
        out
    }

    /// What a body standing in column `x, y` rests on and bumps into, when its
    /// step reaches `reach` cells up the box. `None` off the box.
    ///
    /// Solid at the height of the step is a wall, and the wall's top is the
    /// floor it offers, which a walker refuses when it is more than a step up.
    /// Air there means the first solid below is the floor, and the first
    /// solid above is the roof.
    pub fn gap(&self, x: i32, y: i32, reach: f64) -> Option<Gap> {
        if !self.over(x, y) {
            return None;
        }
        let top = self.size[2] as i32;
        let probe = (reach.floor() as i32).clamp(-1, top);
        let solid = |z: i32| self.solid([x, y, z]);
        if solid(probe) {
            let mut z = probe;
            while z < top && solid(z) {
                z += 1;
            }
            let ceiling = (z..top).find(|&z| solid(z));
            return Some(Gap {
                floor: Some(z),
                ceiling,
            });
        }
        let floor = (0..probe).rev().find(|&z| solid(z)).map(|z| z + 1);
        let ceiling = (probe + 1..top).find(|&z| solid(z));
        Some(Gap { floor, ceiling })
    }

    /// Copies the cells of `span`, a box of this volume, into `out`, which
    /// holds the box `frame` in [`Span::cells`] order with the volume's
    /// first cell at `origin` of it. A row of a chunk is copied whole, and
    /// a chunk with nothing stored is left as `out` has it.
    pub(crate) fn read(&self, span: Span, origin: [i32; 3], frame: Span, out: &mut [Cell]) {
        let Some(span) = span.meet(self.bounds()) else {
            return;
        };
        for index in self.chunks_in(span) {
            let Some(held) = self.chunk_span(index).meet(span) else {
                continue;
            };
            let (slot, _) = self.locate(held.min);
            let Some(chunk) = &self.chunks[slot] else {
                continue;
            };
            let run = (held.max[0] - held.min[0] + 1) as usize;
            for z in held.min[2]..=held.max[2] {
                for y in held.min[1]..=held.max[1] {
                    let (_, from) = self.locate([held.min[0], y, z]);
                    let at = [held.min[0] + origin[0], y + origin[1], z + origin[2]];
                    let to = frame.index(at);
                    out[to..to + run].copy_from_slice(&chunk.cells[from..from + run]);
                }
            }
        }
    }

    /// Chunk slot and index within it of a place inside the box.
    fn locate(&self, at: [i32; 3]) -> (usize, usize) {
        let [cx, cy, _] = self.chunk_count();
        let at = at.map(|n| n as u32);
        let chunk = at.map(|n| n >> CHUNK_BITS);
        let local = at.map(|n| n & (CHUNK - 1));
        let slot = (chunk[2] * cy + chunk[1]) * cx + chunk[0];
        let index = (local[2] * CHUNK + local[1]) * CHUNK + local[0];
        (slot as usize, index as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_pack_into_runs_and_come_back_the_same() {
        let mut cells = vec![Cell::AIR; 4096];
        cells[..256].fill(Cell::solid(3));
        cells[4000] = Cell::solid(200);
        let packed = pack(&cells);
        assert!(packed.len() < 80, "{}", packed.len());
        assert_eq!(unpack(&packed, cells.len()), Some(cells));
        assert_eq!(unpack(&pack(&[]), 0), Some(Vec::new()));
        // Too few, too many, a run of none and half a run are no cells.
        assert_eq!(unpack(&packed, 4095), None);
        assert_eq!(unpack(&packed, 4097), None);
        assert_eq!(unpack(&[0, 1], 0), None);
        assert_eq!(unpack(&[1], 1), None);
    }

    #[test]
    fn a_cell_keeps_its_paint_and_air_has_none() {
        assert_eq!(Cell::AIR.paint(), None);
        assert_eq!(Cell::solid(0).paint(), Some(0));
        assert_eq!(Cell::solid(7).paint(), Some(7));
        assert!(!Cell::solid(254).is_air());
    }

    #[test]
    fn an_emptied_chunk_is_dropped() {
        let mut volume = Volume::new([40, 40, 40]);
        assert!(volume.set([20, 3, 5], Cell::solid(1)));
        assert!(volume.chunk_stored([1, 0, 0]));
        assert!(volume.set([20, 3, 5], Cell::AIR));
        assert!(!volume.chunk_stored([1, 0, 0]));
    }

    #[test]
    fn a_chunk_outside_the_box_is_never_stored() {
        let mut volume = Volume::new([40, 20, 16]);
        assert!(volume.set([0, 16, 0], Cell::solid(1)));
        assert!(volume.chunk_stored([0, 1, 0]));
        assert!(!volume.chunk_stored([3, 0, 0]));
        assert!(!volume.chunk_stored([0, 2, 0]));
        assert!(!volume.chunk_stored([0, 0, 1]));
    }

    #[test]
    fn outside_the_box_is_air_and_so_is_under_it() {
        let volume = Volume::new([4, 4, 4]);
        assert_eq!(volume.get([9, 0, 0]), Cell::AIR);
        assert!(!volume.solid([2, 2, -1]));
        assert!(!volume.solid([2, 2, 0]));
    }

    #[test]
    fn a_gap_is_the_floor_under_and_the_roof_over() {
        let mut volume = Volume::new([8, 8, 16]);
        // A step, a wall two high, and a slab overhead.
        volume.set([1, 0, 0], Cell::solid(0));
        volume.set([2, 0, 0], Cell::solid(0));
        volume.set([2, 0, 1], Cell::solid(0));
        volume.set([3, 0, 9], Cell::solid(0));
        let reach = 1.0;
        // Nothing under the feet: a volume holds up only what is built.
        assert_eq!(
            volume.gap(0, 0, reach),
            Some(Gap {
                floor: None,
                ceiling: None
            })
        );
        assert_eq!(volume.gap(1, 0, reach).unwrap().floor, Some(1));
        assert_eq!(volume.gap(2, 0, reach).unwrap().floor, Some(2));
        assert_eq!(
            volume.gap(3, 0, reach),
            Some(Gap {
                floor: None,
                ceiling: Some(9)
            })
        );
        assert_eq!(volume.gap(8, 0, reach), None);
    }

    #[test]
    fn chunks_in_a_span_are_clipped_to_the_box() {
        let volume = Volume::new([40, 20, 16]);
        assert_eq!(volume.chunk_count(), [3, 2, 1]);
        let span = Span::between([-3, -3, -3], [16, 2, 2]);
        assert_eq!(volume.chunks_in(span), vec![[0, 0, 0], [1, 0, 0]]);
        assert_eq!(volume.chunk_span([2, 1, 0]).max, [39, 19, 15]);
    }
}
