//! Which cell a line of sight meets.
//!
//! A line of sight is a path of points in the volume's own cells. It is a
//! polyline rather than a ray because a volume is seated on a body by whoever
//! holds it, and a straight line in the world is only nearly straight in the
//! cells of a curved one: the caller samples it as finely as it needs, and each
//! piece is walked exactly, cell by cell (Amanatides and Woo), so no cell a
//! piece passes through is ever skipped.

use crate::{Cells, Face, Volume};

/// Where a line of sight stops: a solid cell, and the side of it the line
/// came in through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hit {
    pub cell: [i32; 3],
    pub face: Face,
}

impl Hit {
    /// The air cell in front of the side that was hit: where a new cell goes.
    pub fn before(self) -> [i32; 3] {
        let n = self.face.normal();
        [0, 1, 2].map(|i| self.cell[i] + n[i])
    }
}

impl Volume {
    /// The first solid the path meets after leaving any solid it starts in:
    /// a camera inside a wall looks out of it, not at it.
    pub fn trace(&self, path: &[[f64; 3]]) -> Option<Hit> {
        trace(self, path)
    }
}

/// The first solid a path meets in some cells, after leaving any solid it
/// starts in.
pub(crate) fn trace(cells: &impl Cells, path: &[[f64; 3]]) -> Option<Hit> {
    let mut in_air = path.first().is_some_and(|&p| !cells.solid(cell_of(p)));
    for piece in path.windows(2) {
        let (from, to) = (piece[0], piece[1]);
        let mut cell = cell_of(from);
        let delta = [0, 1, 2].map(|i| to[i] - from[i]);
        let step = delta.map(|d| if d > 0.0 { 1 } else { -1 });
        // How far along the piece, `0..=1`, the next boundary on each axis
        // lies, and how far apart boundaries are.
        let mut next = [0, 1, 2].map(|i| {
            let d = delta[i];
            if d > 0.0 {
                (f64::from(cell[i] + 1) - from[i]) / d
            } else if d < 0.0 {
                (f64::from(cell[i]) - from[i]) / d
            } else {
                f64::INFINITY
            }
        });
        let apart = delta.map(|d| {
            if d == 0.0 {
                f64::INFINITY
            } else {
                1.0 / d.abs()
            }
        });
        loop {
            let axis = (0..3)
                .min_by(|&a, &b| next[a].total_cmp(&next[b]))
                .expect("three axes");
            if next[axis] > 1.0 {
                break;
            }
            cell[axis] += step[axis];
            next[axis] += apart[axis];
            let solid = cells.solid(cell);
            if solid && in_air {
                return Some(Hit {
                    cell,
                    face: Face::new(axis, step[axis] < 0),
                });
            }
            in_air |= !solid;
        }
    }
    None
}

/// The cell a point is in.
fn cell_of(p: [f64; 3]) -> [i32; 3] {
    p.map(|n| n.floor() as i32)
}

/// Where a path first crosses the plane `point[axis] == at`, if it does.
///
/// What a drag holds on to: once a stroke has started on a layer of cells, the
/// pointer is read where its line of sight meets that layer, so the stroke
/// stays flat whatever is behind it, sky included.
pub fn crossing(path: &[[f64; 3]], axis: usize, at: f64) -> Option<[f64; 3]> {
    path.windows(2).find_map(|piece| {
        let (a, b) = (piece[0][axis] - at, piece[1][axis] - at);
        if a == 0.0 {
            return Some(piece[0]);
        }
        if a * b > 0.0 || a == b {
            return None;
        }
        let t = a / (a - b);
        Some([0, 1, 2].map(|i| piece[0][i] + (piece[1][i] - piece[0][i]) * t))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Gesture, Span};

    fn with(cells: &[[i32; 3]]) -> Volume {
        let mut volume = Volume::new([16, 16, 16]);
        for &at in cells {
            volume.apply(Gesture::Create {
                span: Span::cell(at),
                paint: 0,
            });
        }
        volume
    }

    #[test]
    fn a_ray_along_x_meets_the_near_side() {
        let volume = with(&[[5, 2, 3]]);
        let hit = volume
            .trace(&[[0.5, 2.5, 3.5], [12.5, 2.5, 3.5]])
            .expect("hit");
        assert_eq!(hit.cell, [5, 2, 3]);
        assert_eq!(hit.face, Face::new(0, false));
        assert_eq!(hit.before(), [4, 2, 3]);
    }

    #[test]
    fn looking_down_meets_the_top_of_a_cell() {
        let volume = with(&[[3, 4, 0]]);
        let hit = volume
            .trace(&[[3.2, 4.7, 6.0], [3.2, 4.7, -3.0]])
            .expect("hit");
        assert_eq!(hit.cell, [3, 4, 0]);
        assert_eq!(hit.face, Face::UP);
        assert_eq!(hit.before(), [3, 4, 1]);
    }

    #[test]
    fn a_line_through_air_meets_nothing() {
        let volume = with(&[]);
        assert_eq!(volume.trace(&[[3.2, 4.7, 6.0], [3.2, 4.7, -3.0]]), None);
    }

    #[test]
    fn a_path_in_pieces_misses_nothing_between_them() {
        let volume = with(&[[7, 7, 7]]);
        let path: Vec<[f64; 3]> = (0..=40)
            .map(|i| {
                let t = f64::from(i) / 40.0;
                [0.5 + 14.0 * t, 0.5 + 14.0 * t, 0.5 + 14.0 * t]
            })
            .collect();
        assert_eq!(volume.trace(&path).map(|hit| hit.cell), Some([7, 7, 7]));
    }

    #[test]
    fn a_camera_inside_a_wall_looks_out_of_it() {
        let volume = with(&[[2, 2, 2], [6, 2, 2]]);
        let hit = volume
            .trace(&[[2.5, 2.5, 2.5], [12.5, 2.5, 2.5]])
            .expect("hit");
        assert_eq!(hit.cell, [6, 2, 2]);
    }

    #[test]
    fn a_path_crosses_a_layer_once() {
        let path = [[0.0, 0.0, 10.0], [4.0, 2.0, 6.0], [8.0, 4.0, 2.0]];
        assert_eq!(crossing(&path, 2, 4.0), Some([6.0, 3.0, 4.0]));
        assert_eq!(crossing(&path, 2, 12.0), None);
    }
}
