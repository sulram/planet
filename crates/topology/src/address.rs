//! Integer addresses and neighbours across sector seams.
//!
//! Seams are not a lookup table. A column maps to an integer point on the
//! surface of a cube with edge `2 * side` (doubled coordinates, so cell
//! centres are odd integers and faces sit at the even `+-side`). A step over
//! an edge is one vector sum on that cube, and the point maps back to a
//! column. Swaps and flips of `u` and `v` fall out of the sector frames.
//!
//! A [`Column`] carries no size. The same integers name a cell on any grid,
//! and what they mean is the [`Grid`] they are asked about.

use crate::grid::Grid;
use crate::sector::{Dir, Sector, idot};

/// A surface cell: one column of blocks.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Column {
    pub sector: Sector,
    pub u: u16,
    pub v: u16,
}

/// The result of one grid step: where you land and which way you now face.
///
/// Across a seam the grid axes of the two sectors differ, so "straight ahead"
/// is a different [`Dir`] on the other side.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Step {
    pub column: Column,
    pub dir: Dir,
}

impl Column {
    pub fn new(sector: Sector, u: u16, v: u16) -> Column {
        Column { sector, u, v }
    }
}

impl Grid {
    /// Centre of the cell on the doubled integer cube.
    fn cube_point(self, column: Column) -> [i32; 3] {
        let n_side = self.side() as i32;
        let (n, ua, va) = (
            column.sector.normal(),
            column.sector.u_axis(),
            column.sector.v_axis(),
        );
        let a = 2 * i32::from(column.u) + 1 - n_side;
        let b = 2 * i32::from(column.v) + 1 - n_side;
        [0, 1, 2].map(|i| n[i] * n_side + ua[i] * a + va[i] * b)
    }

    /// Inverse of [`Grid::cube_point`]. Cell centres are odd on the two
    /// in-face axes, so exactly one component has magnitude `side`.
    fn column_at(self, p: [i32; 3]) -> Column {
        let n_side = self.side() as i32;
        let axis = p
            .iter()
            .position(|c| c.abs() == n_side)
            .expect("a point on the cube surface");
        let mut n = [0; 3];
        n[axis] = p[axis].signum();
        let sector = Sector::from_normal(n);
        let a = idot(p, sector.u_axis());
        let b = idot(p, sector.v_axis());
        Column {
            sector,
            u: ((a + n_side - 1) / 2) as u16,
            v: ((b + n_side - 1) / 2) as u16,
        }
    }

    /// One step along the grid, resolved across seams.
    pub fn step(self, column: Column, dir: Dir) -> Step {
        let n_side = self.side() as i32;
        let (du, dv) = dir.delta();
        let (u, v) = (i32::from(column.u) + du, i32::from(column.v) + dv);
        if (0..n_side).contains(&u) && (0..n_side).contains(&v) {
            return Step {
                column: Column {
                    sector: column.sector,
                    u: u as u16,
                    v: v as u16,
                },
                dir,
            };
        }

        // Over the edge: the travel direction `t` becomes the new normal, and
        // the cell lands one step down the new face: `p + t - n`.
        let (n, ua, va) = (
            column.sector.normal(),
            column.sector.u_axis(),
            column.sector.v_axis(),
        );
        let t = [0, 1, 2].map(|i| ua[i] * du + va[i] * dv);
        let p = self.cube_point(column);
        let landed = self.column_at([0, 1, 2].map(|i| p[i] + t[i] - n[i]));

        // Straight ahead now runs down the new face, against the old normal.
        let heading = n.map(|c| -c);
        let dir = Dir::from_delta(
            idot(heading, landed.sector.u_axis()),
            idot(heading, landed.sector.v_axis()),
        );
        Step {
            column: landed,
            dir,
        }
    }

    /// The four edge neighbours of a column, in [`Dir::ALL`] order.
    pub fn neighbours(self, column: Column) -> [Column; 4] {
        Dir::ALL.map(|dir| self.step(column, dir).column)
    }
}

/// A block: a column plus a height in blocks from the datum sphere.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Address {
    pub column: Column,
    pub h: i16,
}

impl Address {
    pub fn new(column: Column, h: i16) -> Address {
        Address { column, h }
    }
}
