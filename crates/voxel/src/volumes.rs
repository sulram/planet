//! Volumes side by side, read as one.
//!
//! The frame is cut into plots, the squares of a fixed grid, and a volume
//! stands over one plot, as low and as tall as whoever opens it says. They
//! share the frame: `x` and `y` across, cut every [`Volumes::side`] cells,
//! and `z` up, cut by the chunk. A gesture, a line of sight, the sides that
//! show and a footing read across the plots as if the cells were one store,
//! so what is too big for one volume stands over two neighbours. Off every
//! open plot there is nothing.

use std::collections::BTreeMap;

use crate::faces::{faces_in, faces_into};
use crate::trace::trace;
use crate::{CHUNK, CHUNK_BITS, Cell, Cells, Gap, Gesture, Hit, Quad, Span, Volume};

/// A volume over its plot, and where its first cell is in the frame: the
/// corner of the plot, at the height the volume starts from.
#[derive(Clone)]
struct Standing {
    origin: [i32; 3],
    volume: Volume,
}

impl Standing {
    /// A place of the frame in the volume's own cells.
    fn local(&self, at: [i32; 3]) -> [i32; 3] {
        [0, 1, 2].map(|i| at[i] - self.origin[i])
    }

    /// A box of the frame in the volume's own cells.
    fn local_span(&self, span: Span) -> Span {
        span.moved(self.origin.map(|n| -n))
    }

    /// The cells of the volume, in the frame.
    fn bounds(&self) -> Span {
        self.volume.bounds().moved(self.origin)
    }
}

/// Volumes over the plots of a fixed grid, in one frame.
#[derive(Clone)]
pub struct Volumes {
    /// Cells along the side of a plot, as a power of two.
    bits: u32,
    standing: BTreeMap<[i32; 2], Standing>,
}

impl core::fmt::Debug for Volumes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Volumes")
            .field("side", &self.side())
            .field("plots", &self.standing.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Volumes {
    /// No volume yet, over plots of `2^plot_bits` cells a side. A plot holds
    /// at least a chunk.
    pub fn new(plot_bits: u32) -> Volumes {
        Volumes {
            bits: plot_bits.max(CHUNK_BITS),
            standing: BTreeMap::new(),
        }
    }

    /// Cells along the side of a plot.
    pub fn side(&self) -> u32 {
        1 << self.bits
    }

    /// The plot a column is on.
    pub fn plot_of(&self, x: i32, y: i32) -> [i32; 2] {
        [x >> self.bits, y >> self.bits]
    }

    /// Opens a volume over a plot, holding the cells from `low` up to
    /// `height` over it, and as many more as bring its bottom and its top to
    /// a chunk of the frame: a chunk is cut by the frame along `z` as along
    /// `x` and `y`. False where a volume stands already, which stays as it is.
    pub fn open(&mut self, plot: [i32; 2], low: i32, height: u32) -> bool {
        if self.standing.contains_key(&plot) {
            return false;
        }
        let chunk = CHUNK as i32;
        let bottom = low.div_euclid(chunk) * chunk;
        let top = (low + height.max(1) as i32 + chunk - 1).div_euclid(chunk) * chunk;
        let side = self.side();
        let standing = Standing {
            origin: [plot[0] << self.bits, plot[1] << self.bits, bottom],
            volume: Volume::new([side, side, (top - bottom) as u32]),
        };
        self.standing.insert(plot, standing);
        true
    }

    /// Whether a volume stands over a plot.
    pub fn is_open(&self, plot: [i32; 2]) -> bool {
        self.standing.contains_key(&plot)
    }

    /// The cells of the volume over a plot.
    pub fn bounds(&self, plot: [i32; 2]) -> Option<Span> {
        self.standing.get(&plot).map(Standing::bounds)
    }

    /// Every plot a volume stands over.
    pub fn plots(&self) -> impl Iterator<Item = [i32; 2]> + '_ {
        self.standing.keys().copied()
    }

    /// The volume a column is on.
    fn over(&self, x: i32, y: i32) -> Option<&Standing> {
        self.standing.get(&self.plot_of(x, y))
    }

    /// The plots a box reaches over, open or not.
    fn plots_in(&self, span: Span) -> impl Iterator<Item = [i32; 2]> + use<> {
        let lo = [span.min[0] >> self.bits, span.min[1] >> self.bits];
        let hi = [span.max[0] >> self.bits, span.max[1] >> self.bits];
        (lo[1]..=hi[1]).flat_map(move |y| (lo[0]..=hi[0]).map(move |x| [x, y]))
    }

    /// The part of a box that volumes hold: the box around every cell of
    /// theirs in it, `None` when it reaches into no volume.
    pub fn held(&self, span: Span) -> Option<Span> {
        self.plots_in(span)
            .filter_map(|plot| span.meet(self.standing.get(&plot)?.bounds()))
            .reduce(|held, part| join(Some(held), part))
    }

    /// Whether a place is a cell of some volume, air or not.
    pub fn holds(&self, at: [i32; 3]) -> bool {
        self.over(at[0], at[1])
            .is_some_and(|standing| standing.volume.contains(standing.local(at)))
    }

    /// The cell at a place, air where no volume has one.
    pub fn get(&self, at: [i32; 3]) -> Cell {
        self.over(at[0], at[1]).map_or(Cell::AIR, |standing| {
            standing.volume.get(standing.local(at))
        })
    }

    /// Whether a place is solid to a ray, a body or the light: a cell that is
    /// not air.
    pub fn solid(&self, at: [i32; 3]) -> bool {
        !self.get(at).is_air()
    }

    /// Applies a gesture to every volume its box reaches into. Returns the
    /// box around every cell it changed, `None` when it changed nothing.
    pub fn apply(&mut self, gesture: Gesture) -> Option<Span> {
        let mut changed: Option<Span> = None;
        for plot in self.plots_in(gesture.span()) {
            let Some(standing) = self.standing.get_mut(&plot) else {
                continue;
            };
            let local = gesture.over(standing.local_span(gesture.span()));
            if let Some(span) = standing.volume.apply(local) {
                changed = Some(join(changed, span.moved(standing.origin)));
            }
        }
        changed
    }

    /// The cells of a box in [`Span::cells`] order, air where no volume has
    /// one: what a gesture is about to change, kept to take it back.
    pub fn cells(&self, span: Span) -> Vec<Cell> {
        let mut cells = vec![Cell::AIR; span.cells().count()];
        for plot in self.plots_in(span) {
            let Some(standing) = self.standing.get(&plot) else {
                continue;
            };
            let Some(held) = span.meet(standing.bounds()) else {
                continue;
            };
            for at in held.cells() {
                cells[index_in(span, at)] = standing.volume.get(standing.local(at));
            }
        }
        cells
    }

    /// Puts back cells [`Volumes::cells`] read from the same box. Returns the
    /// box around every cell it changed, `None` when it changed nothing.
    pub fn restore(&mut self, span: Span, cells: &[Cell]) -> Option<Span> {
        let mut changed: Option<Span> = None;
        for plot in self.plots_in(span) {
            let Some(standing) = self.standing.get_mut(&plot) else {
                continue;
            };
            let Some(held) = span.meet(standing.bounds()) else {
                continue;
            };
            for at in held.cells() {
                let Some(&cell) = cells.get(index_in(span, at)) else {
                    continue;
                };
                if standing.volume.set(standing.local(at), cell) {
                    changed = Some(join(changed, Span::cell(at)));
                }
            }
        }
        changed
    }

    /// The first solid the path meets after leaving any solid it starts in,
    /// whichever volume holds it.
    pub fn trace(&self, path: &[[f64; 3]]) -> Option<Hit> {
        trace(self, path)
    }

    /// Every chunk a box of cells reaches into, each named by its lowest
    /// corner in the frame.
    pub fn chunks_in(&self, span: Span) -> Vec<[i32; 3]> {
        let mut out = Vec::new();
        for plot in self.plots_in(span) {
            let Some(standing) = self.standing.get(&plot) else {
                continue;
            };
            let chunks = standing.volume.chunks_in(standing.local_span(span));
            out.extend(chunks.into_iter().map(|chunk| {
                let local = chunk.map(|c| (c * CHUNK) as i32);
                [0, 1, 2].map(|i| standing.origin[i] + local[i])
            }));
        }
        out
    }

    /// The visible sides of the cells of one chunk, named by its lowest
    /// corner. A side against a solid cell of the volume next door is hidden
    /// as one against its own.
    pub fn faces(&self, chunk: [i32; 3]) -> Vec<Quad> {
        let Some(standing) = self.over(chunk[0], chunk[1]) else {
            return Vec::new();
        };
        let local = standing.local(chunk);
        if local.iter().any(|&n| n < 0 || n as u32 % CHUNK != 0) {
            return Vec::new();
        }
        let index = local.map(|n| n as u32 >> CHUNK_BITS);
        if !standing.volume.chunk_stored(index) {
            return Vec::new();
        }
        let span = standing.volume.chunk_span(index).moved(standing.origin);
        let home = Home {
            volumes: self,
            standing,
        };
        faces_in(&home, span)
    }

    /// The sides of exactly the cells a gesture would change, as if nothing
    /// else stood: what its ghost is made of. Creating takes air; deleting
    /// and painting take what is solid.
    pub fn ghost(&self, gesture: Gesture) -> Vec<Quad> {
        let span = gesture.span();
        let mut quads = Vec::new();
        for plot in self.plots_in(span) {
            let Some(standing) = self.standing.get(&plot) else {
                continue;
            };
            let Some(held) = span.meet(standing.bounds()) else {
                continue;
            };
            let taken = Taken {
                volumes: self,
                span,
                takes_air: gesture.takes_air(),
            };
            faces_into(&taken, held, &mut quads);
        }
        quads
    }

    /// What a body standing in column `x, y` rests on and bumps into, when
    /// its step reaches `reach` cells up the frame. `None` off every volume.
    pub fn gap(&self, x: i32, y: i32, reach: f64) -> Option<Gap> {
        let standing = self.over(x, y)?;
        let [x0, y0, low] = standing.origin;
        let gap = standing
            .volume
            .gap(x - x0, y - y0, reach - f64::from(low))?;
        Some(Gap {
            floor: gap.floor.map(|z| z + low),
            ceiling: gap.ceiling.map(|z| z + low),
        })
    }

    /// Whether a column holds a solid cell with any part of it between two
    /// heights: what stands in the way of something that tall.
    pub fn covers(&self, x: i32, y: i32, from: f64, to: f64) -> bool {
        (from.floor() as i32..=to.floor() as i32).any(|z| self.solid([x, y, z]))
    }
}

impl Cells for Volumes {
    fn get(&self, at: [i32; 3]) -> Cell {
        Volumes::get(self, at)
    }
}

/// The cells as the chunks of one volume read them: its own at once, and a
/// neighbour's looked up only past the edge of the plot.
struct Home<'a> {
    volumes: &'a Volumes,
    standing: &'a Standing,
}

impl Cells for Home<'_> {
    fn get(&self, at: [i32; 3]) -> Cell {
        let local = self.standing.local(at);
        if self.standing.volume.over(local[0], local[1]) {
            self.standing.volume.get(local)
        } else {
            self.volumes.get(at)
        }
    }
}

/// The cells a gesture would take, each one solid, and nothing else.
struct Taken<'a> {
    volumes: &'a Volumes,
    span: Span,
    takes_air: bool,
}

impl Cells for Taken<'_> {
    fn get(&self, at: [i32; 3]) -> Cell {
        let taken = self.span.contains(at)
            && self.volumes.holds(at)
            && self.volumes.get(at).is_air() == self.takes_air;
        if taken { Cell::solid(0) } else { Cell::AIR }
    }
}

/// A box grown just enough to hold another.
fn join(span: Option<Span>, other: Span) -> Span {
    span.map_or(other, |span| span.with(other.min).with(other.max))
}

/// Where a cell of a box comes in [`Span::cells`] order.
fn index_in(span: Span, at: [i32; 3]) -> usize {
    let [sx, sy, _] = span.size().map(|n| n as usize);
    let [x, y, z] = [0, 1, 2].map(|i| (at[i] - span.min[i]) as usize);
    (z * sy + y) * sx + x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    /// Two volumes side by side along `x`, 32 cells a side: the first from
    /// the bottom of the frame and 32 tall, the second from `low` and as
    /// tall.
    fn pair(low: i32) -> Volumes {
        let mut volumes = Volumes::new(5);
        assert!(volumes.open([0, 0], 0, 32));
        assert!(volumes.open([1, 0], low, 32));
        volumes
    }

    fn create(volumes: &mut Volumes, a: [i32; 3], b: [i32; 3], paint: u8) -> Option<Span> {
        volumes.apply(Gesture::Create {
            span: Span::between(a, b),
            paint,
        })
    }

    #[test]
    fn a_plot_is_cut_by_the_frame_and_opens_once() {
        let mut volumes = Volumes::new(5);
        assert_eq!(volumes.plot_of(31, 32), [0, 1]);
        assert_eq!(volumes.plot_of(-1, 0), [-1, 0]);
        assert!(volumes.open([2, -1], 7, 20));
        assert!(!volumes.open([2, -1], 90, 20));
        assert!(volumes.is_open([2, -1]));
        assert!(!volumes.is_open([0, 0]));
        // From 7 to 27, and out to the chunks that hold them.
        assert_eq!(
            volumes.bounds([2, -1]),
            Some(Span::between([64, -32, 0], [95, -1, 31]))
        );
        assert!(volumes.open([0, 0], -20, 20));
        let bounds = volumes.bounds([0, 0]).unwrap();
        assert_eq!((bounds.min[2], bounds.max[2]), (-32, -1));
    }

    #[test]
    fn a_gesture_stands_over_two_neighbours() {
        let mut volumes = pair(0);
        let changed = create(&mut volumes, [28, 3, 0], [35, 3, 0], 4);
        assert_eq!(changed, Some(Span::between([28, 3, 0], [35, 3, 0])));
        for x in 28..=35 {
            assert_eq!(volumes.get([x, 3, 0]).paint(), Some(4), "{x}");
        }
    }

    #[test]
    fn off_every_plot_there_is_nothing() {
        let mut volumes = pair(0);
        let changed = create(&mut volumes, [60, 3, 0], [70, 3, 0], 1);
        assert_eq!(changed, Some(Span::between([60, 3, 0], [63, 3, 0])));
        assert!(volumes.get([64, 3, 0]).is_air());
        assert!(!volumes.holds([64, 3, 0]));
        assert!(!volumes.holds([63, 3, -1]));
        assert_eq!(volumes.gap(64, 3, 1.0), None);
    }

    #[test]
    fn the_side_between_two_volumes_is_hidden() {
        let mut volumes = pair(0);
        create(&mut volumes, [31, 3, 0], [32, 3, 0], 0);
        let mut quads = volumes.faces([16, 0, 0]);
        quads.extend(volumes.faces([32, 0, 0]));
        // Two cubes in a row: six sides each, less the one between.
        assert_eq!(quads.len(), 10);
        assert!(
            !quads
                .iter()
                .any(|q| q.cell == [31, 3, 0] && q.face == Face::new(0, true))
        );
        assert!(
            !quads
                .iter()
                .any(|q| q.cell == [32, 3, 0] && q.face == Face::new(0, false))
        );
    }

    #[test]
    fn a_corner_is_shut_in_by_the_volume_next_door() {
        let mut volumes = pair(0);
        create(&mut volumes, [31, 3, 0], [31, 3, 0], 0);
        create(&mut volumes, [32, 3, 1], [32, 3, 1], 0);
        let top = volumes
            .faces([16, 0, 0])
            .into_iter()
            .find(|q| q.cell == [31, 3, 0] && q.face == Face::UP)
            .expect("the top of the cube");
        for (corner, open) in top.corners().into_iter().zip(top.open) {
            assert_eq!(open < 3, corner[0] == 32, "{corner:?} {open}");
        }
    }

    #[test]
    fn volumes_that_start_at_different_heights_are_read_as_one() {
        // The second starts a chunk higher: what both hold meets, and what
        // only one holds is its own.
        let mut volumes = pair(16);
        create(&mut volumes, [31, 3, 20], [32, 3, 20], 0);
        let mut quads = volumes.faces([16, 0, 16]);
        quads.extend(volumes.faces([32, 0, 16]));
        assert_eq!(quads.len(), 10);
        let changed = create(&mut volumes, [31, 3, 4], [32, 3, 4], 0);
        assert_eq!(changed, Some(Span::cell([31, 3, 4])));
        assert_eq!(volumes.gap(31, 3, 5.0).unwrap().floor, Some(5));
        assert_eq!(volumes.gap(32, 3, 5.0).unwrap().floor, None);
        assert_eq!(volumes.gap(32, 3, 21.0).unwrap().floor, Some(21));
    }

    #[test]
    fn a_line_of_sight_passes_from_one_volume_into_the_next() {
        let mut volumes = pair(0);
        create(&mut volumes, [40, 3, 2], [40, 3, 2], 0);
        let hit = volumes
            .trace(&[[20.5, 3.5, 2.5], [50.5, 3.5, 2.5]])
            .expect("hit");
        assert_eq!(hit.cell, [40, 3, 2]);
        assert_eq!(hit.face, Face::new(0, false));
        assert_eq!(volumes.trace(&[[30.5, 3.5, 4.0], [36.5, 3.5, -2.0]]), None);
    }

    #[test]
    fn cells_read_across_two_volumes_take_a_gesture_back() {
        let mut volumes = pair(0);
        create(&mut volumes, [30, 3, 0], [30, 3, 0], 5);
        let span = Span::between([28, 3, 0], [70, 3, 0]);
        let before = volumes.cells(span);
        assert_eq!(before.len(), 43);
        create(&mut volumes, span.min, span.max, 2);
        let after = volumes.cells(span);
        assert_eq!(
            volumes.restore(span, &before),
            Some(Span::between([28, 3, 0], [63, 3, 0]))
        );
        assert_eq!(volumes.get([30, 3, 0]).paint(), Some(5));
        assert!(volumes.get([33, 3, 0]).is_air());
        volumes.restore(span, &after);
        assert_eq!(volumes.get([33, 3, 0]).paint(), Some(2));
        assert_eq!(volumes.restore(span, &after), None);
    }

    #[test]
    fn what_is_held_of_a_box_is_what_volumes_have_of_it() {
        let volumes = pair(16);
        let span = Span::between([-9, 3, -5], [200, 3, 40]);
        assert_eq!(
            volumes.held(span),
            Some(Span::between([0, 3, 0], [63, 3, 40]))
        );
        assert_eq!(volumes.held(Span::cell([70, 3, 0])), None);
    }

    #[test]
    fn chunks_are_named_by_their_corner_in_the_frame() {
        let mut volumes = pair(16);
        let span = Span::between([30, 0, 14], [33, 0, 17]);
        assert_eq!(
            volumes.chunks_in(span),
            vec![[16, 0, 0], [16, 0, 16], [32, 0, 16]]
        );
        create(&mut volumes, [33, 0, 17], [33, 0, 17], 0);
        assert_eq!(volumes.faces([32, 0, 16]).len(), 6);
        assert!(volumes.faces([32, 0, 0]).is_empty());
        assert!(volumes.faces([16, 0, 16]).is_empty());
        assert!(volumes.faces([32, 0, 17]).is_empty());
    }

    #[test]
    fn a_ghost_is_the_cells_a_gesture_would_change() {
        let mut volumes = pair(0);
        create(&mut volumes, [31, 3, 0], [32, 3, 0], 0);
        let span = Span::between([30, 3, 0], [33, 3, 0]);
        // Creating takes the air either side of the two cubes: two cells
        // apart, six sides each.
        let ghost = volumes.ghost(Gesture::Create { span, paint: 1 });
        assert_eq!(ghost.len(), 12);
        assert!(ghost.iter().all(|q| q.cell[0] == 30 || q.cell[0] == 33));
        // Deleting takes the cubes, across the two volumes as one: ten.
        assert_eq!(volumes.ghost(Gesture::Delete { span }).len(), 10);
        let off = Span::between([64, 3, 0], [70, 3, 0]);
        assert!(volumes.ghost(Gesture::Delete { span: off }).is_empty());
    }

    #[test]
    fn a_column_is_covered_where_a_cell_stands_in_the_way() {
        let mut volumes = pair(0);
        create(&mut volumes, [5, 5, 9], [5, 5, 9], 0);
        assert!(volumes.covers(5, 5, 8.2, 9.1));
        assert!(volumes.covers(5, 5, 9.9, 12.0));
        assert!(!volumes.covers(5, 5, 7.0, 8.9));
        assert!(!volumes.covers(5, 5, 10.0, 12.0));
        assert!(!volumes.covers(6, 5, 8.2, 9.1));
    }
}
