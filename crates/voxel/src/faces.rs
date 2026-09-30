//! The sides of cells that show, and how shut in each of their corners is.
//!
//! Every side of a solid cell that looks at air is one quad; sides between
//! two solids, across two volumes too, are never drawn. Sides
//! are not merged into larger quads: whoever seats a volume on a curved body
//! bends each corner onto it, and a merged quad would be a chord whose
//! neighbours meet it in the middle of an edge, where the surface cracks.
//!
//! Occlusion is per corner, from the three cells that touch the corner in the
//! layer the side looks into: two along the edges and one across. It is the
//! cheap half of the light a volume is meant to bake, and what makes a corner
//! of a room read as one.

use crate::{Cell, Cells, Face, Span, Volume};

/// One visible side of one cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Quad {
    pub cell: [i32; 3],
    pub face: Face,
    pub paint: u8,
    /// How open each corner is, in the order of [`Quad::corners`]: `3` open,
    /// `0` shut in on both edges.
    pub open: [u8; 4],
}

impl Quad {
    /// The corners of the side as lattice points, counter clockwise seen from
    /// outside when `x`, `y` and `z` make a right handed frame.
    pub fn corners(self) -> [[i32; 3]; 4] {
        let (a, b, c) = axes(self.face);
        let mut base = self.cell;
        if self.face.positive {
            base[a] += 1;
        }
        let at = |sb: i32, sc: i32| {
            let mut p = base;
            p[b] += sb;
            p[c] += sc;
            p
        };
        let square = [at(0, 0), at(1, 0), at(1, 1), at(0, 1)];
        if self.face.positive {
            square
        } else {
            [square[0], square[3], square[2], square[1]]
        }
    }

    /// Whether the side is split along the diagonal from the second corner to
    /// the fourth instead of from the first to the third. Split along the
    /// brighter diagonal, occlusion shades evenly instead of in a crease.
    pub fn flipped(self) -> bool {
        let [a, b, c, d] = self.open.map(u32::from);
        a + c < b + d
    }
}

/// The axis a side faces, then the two across it in right handed order.
fn axes(face: Face) -> (usize, usize, usize) {
    let a = face.axis;
    (a, (a + 1) % 3, (a + 2) % 3)
}

impl Volume {
    /// The visible sides of the cells of one chunk.
    pub fn faces(&self, chunk: [u32; 3]) -> Vec<Quad> {
        if !self.chunk_stored(chunk) {
            return Vec::new();
        }
        self.faces_in(self.chunk_span(chunk))
    }

    /// The visible sides of the cells in a box.
    pub fn faces_in(&self, span: Span) -> Vec<Quad> {
        faces_in(self, span)
    }
}

/// Cells read once out of wherever they are kept, in [`Span::cells`] order:
/// a chunk and one cell all round is every cell its sides and their corners
/// look at, answered by index instead of by a lookup through the plots.
pub(crate) struct Padded {
    pub span: Span,
    pub cells: Vec<Cell>,
}

impl Cells for Padded {
    fn get(&self, at: [i32; 3]) -> Cell {
        if self.span.contains(at) {
            self.cells[self.span.index(at)]
        } else {
            Cell::AIR
        }
    }
}

impl Padded {
    /// The visible sides of the cells of `span`, which lies a cell inside
    /// the copy all round: what [`faces_into`] finds, with every neighbour a
    /// side and its corners look at read a fixed step away in the copy.
    pub(crate) fn faces(&self, span: Span) -> Vec<Quad> {
        let size = self.span.size().map(|n| n as isize);
        let stride = [1, size[0], size[0] * size[1]];
        let step = |n: [i32; 3]| (0..3).map(|i| n[i] as isize * stride[i]).sum::<isize>();
        let solid = |i: isize| !self.cells[i as usize].is_air();
        let mut quads = Vec::new();
        for at in span.cells() {
            let i = self.span.index(at) as isize;
            let Some(paint) = self.cells[i as usize].paint() else {
                continue;
            };
            for face in Face::ALL {
                let out = i + step(face.normal());
                if solid(out) {
                    continue;
                }
                let (_, b, c) = axes(face);
                let open = openness(face, |sb, sc| {
                    solid(out + sb as isize * stride[b] + sc as isize * stride[c])
                });
                quads.push(Quad {
                    cell: at,
                    face,
                    paint,
                    open,
                });
            }
        }
        quads
    }
}

/// The visible sides of the cells in a box, into `quads`.
pub(crate) fn faces_into(cells: &impl Cells, span: Span, quads: &mut Vec<Quad>) {
    for at in span.cells() {
        let Some(paint) = cells.get(at).paint() else {
            continue;
        };
        for face in Face::ALL {
            let n = face.normal();
            let out = [0, 1, 2].map(|i| at[i] + n[i]);
            if cells.solid(out) {
                continue;
            }
            let (_, b, c) = axes(face);
            let open = openness(face, |sb, sc| {
                let mut p = out;
                p[b] += sb;
                p[c] += sc;
                cells.solid(p)
            });
            quads.push(Quad {
                cell: at,
                face,
                paint,
                open,
            });
        }
    }
}

/// The visible sides of the cells in a box.
pub(crate) fn faces_in(cells: &impl Cells, span: Span) -> Vec<Quad> {
    let mut quads = Vec::new();
    faces_into(cells, span, &mut quads);
    quads
}

/// How open each corner of a side is, from whether the cells beside the
/// one it looks into are solid: `solid(sb, sc)` steps along the two axes
/// across the side.
fn openness(face: Face, solid: impl Fn(i32, i32) -> bool) -> [u8; 4] {
    let corner = |sb: i32, sc: i32| {
        let (edge_b, edge_c) = (solid(sb, 0), solid(0, sc));
        if edge_b && edge_c {
            0
        } else {
            3 - u8::from(edge_b) - u8::from(edge_c) - u8::from(solid(sb, sc))
        }
    };
    let square = [corner(-1, -1), corner(1, -1), corner(1, 1), corner(-1, 1)];
    if face.positive {
        square
    } else {
        [square[0], square[3], square[2], square[1]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gesture;

    fn cross(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    #[test]
    fn corners_wind_counter_clockwise_from_outside() {
        for face in Face::ALL {
            let quad = Quad {
                cell: [3, 4, 5],
                face,
                paint: 0,
                open: [3; 4],
            };
            let [p0, p1, _, p3] = quad.corners();
            let e1 = [0, 1, 2].map(|i| p1[i] - p0[i]);
            let e2 = [0, 1, 2].map(|i| p3[i] - p0[i]);
            assert_eq!(cross(e1, e2), face.normal(), "{face:?}");
        }
    }

    #[test]
    fn a_cube_alone_shows_six_sides() {
        let mut volume = Volume::new([8, 8, 8]);
        volume.apply(Gesture::Create {
            span: Span::cell([2, 2, 0]),
            paint: 3,
        });
        let quads = volume.faces([0, 0, 0]);
        assert_eq!(quads.len(), 6);
        assert!(quads.iter().all(|q| q.paint == 3));
    }

    #[test]
    fn touching_cubes_hide_the_sides_between_them() {
        let mut volume = Volume::new([8, 8, 8]);
        volume.apply(Gesture::Create {
            span: Span::between([1, 1, 2], [2, 1, 2]),
            paint: 0,
        });
        assert_eq!(volume.faces([0, 0, 0]).len(), 10);
    }

    #[test]
    fn a_slab_shuts_in_the_foot_of_a_wall() {
        let mut volume = Volume::new([8, 8, 8]);
        volume.apply(Gesture::Create {
            span: Span::between([0, 0, 0], [7, 7, 0]),
            paint: 0,
        });
        volume.apply(Gesture::Create {
            span: Span::cell([2, 2, 1]),
            paint: 0,
        });
        let side = volume
            .faces([0, 0, 0])
            .into_iter()
            .find(|q| q.cell == [2, 2, 1] && q.face == Face::new(0, true))
            .expect("the +x side");
        for (corner, open) in side.corners().into_iter().zip(side.open) {
            let low = corner[2] == 1;
            assert_eq!(open < 3, low, "{corner:?} {open}");
        }
    }

    #[test]
    fn a_chunk_with_nothing_in_it_has_no_faces() {
        let volume = Volume::new([32, 32, 32]);
        assert!(volume.faces([1, 1, 1]).is_empty());
    }
}
