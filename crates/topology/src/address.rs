//! Integer addresses and neighbours across sector seams.
//!
//! Seams are not a lookup table. A column maps to an integer point on the
//! surface of a cube with edge `2 * SECTOR_SIDE` (doubled coordinates, so cell
//! centres are odd integers and faces sit at the even `+-SECTOR_SIDE`). A step
//! over an edge is one vector sum on that cube, and the point maps back to a
//! column. Swaps and flips of `u` and `v` fall out of the sector frames.

use crate::SECTOR_SIDE;
use crate::sector::{Dir, Sector, idot};

const N: i32 = SECTOR_SIDE as i32;

/// A surface cell: one column of blocks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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

    /// Centre of the cell on the doubled integer cube.
    fn cube_point(self) -> [i32; 3] {
        let (n, ua, va) = (
            self.sector.normal(),
            self.sector.u_axis(),
            self.sector.v_axis(),
        );
        let a = 2 * i32::from(self.u) + 1 - N;
        let b = 2 * i32::from(self.v) + 1 - N;
        [0, 1, 2].map(|i| n[i] * N + ua[i] * a + va[i] * b)
    }

    /// Inverse of [`Column::cube_point`]. Cell centres are odd on the two
    /// in-face axes, so exactly one component has magnitude `N`.
    fn from_cube_point(p: [i32; 3]) -> Column {
        let axis = p
            .iter()
            .position(|c| c.abs() == N)
            .expect("a point on the cube surface");
        let mut n = [0; 3];
        n[axis] = p[axis].signum();
        let sector = Sector::from_normal(n);
        let a = idot(p, sector.u_axis());
        let b = idot(p, sector.v_axis());
        Column {
            sector,
            u: ((a + N - 1) / 2) as u16,
            v: ((b + N - 1) / 2) as u16,
        }
    }

    /// One step along the grid, resolved across seams.
    pub fn step(self, dir: Dir) -> Step {
        let (du, dv) = dir.delta();
        let (u, v) = (i32::from(self.u) + du, i32::from(self.v) + dv);
        if (0..N).contains(&u) && (0..N).contains(&v) {
            return Step {
                column: Column {
                    sector: self.sector,
                    u: u as u16,
                    v: v as u16,
                },
                dir,
            };
        }

        // Over the edge: the travel direction `t` becomes the new normal, and
        // the cell lands one step down the new face: `p + t - n`.
        let (n, ua, va) = (
            self.sector.normal(),
            self.sector.u_axis(),
            self.sector.v_axis(),
        );
        let t = [0, 1, 2].map(|i| ua[i] * du + va[i] * dv);
        let p = self.cube_point();
        let column = Column::from_cube_point([0, 1, 2].map(|i| p[i] + t[i] - n[i]));

        // Straight ahead now runs down the new face, against the old normal.
        let heading = n.map(|c| -c);
        let dir = Dir::from_delta(
            idot(heading, column.sector.u_axis()),
            idot(heading, column.sector.v_axis()),
        );
        Step { column, dir }
    }

    /// The four edge neighbours, in [`Dir::ALL`] order.
    pub fn neighbours(self) -> [Column; 4] {
        Dir::ALL.map(|dir| self.step(dir).column)
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
