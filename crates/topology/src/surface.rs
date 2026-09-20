//! Continuous positions in address space.

use crate::address::Column;
use crate::sector::Sector;
use crate::{SECTOR_SIDE, vec3};

const SIDE: f64 = SECTOR_SIDE as f64;
const HALF: f64 = SIDE / 2.0;

/// A point on the surface in address space: block units, `0..=SECTOR_SIDE`.
///
/// Coordinates outside the range are legal input to [`SurfacePoint::wrapped`],
/// which unfolds them across the seam onto the neighbouring sector.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfacePoint {
    pub sector: Sector,
    pub u: f64,
    pub v: f64,
}

impl SurfacePoint {
    pub fn new(sector: Sector, u: f64, v: f64) -> SurfacePoint {
        SurfacePoint { sector, u, v }
    }

    /// The centre of a column.
    pub fn center_of(column: Column) -> SurfacePoint {
        SurfacePoint {
            sector: column.sector,
            u: f64::from(column.u) + 0.5,
            v: f64::from(column.v) + 0.5,
        }
    }

    /// The column that contains this point. The point must be in range.
    pub fn column(self) -> Column {
        let cell = |x: f64| (libm::floor(x) as i64).clamp(0, i64::from(SECTOR_SIDE) - 1) as u16;
        Column {
            sector: self.sector,
            u: cell(self.u),
            v: cell(self.v),
        }
    }

    /// Brings an out of range point back in range by folding it over the seam
    /// it crossed. Distances in address space are preserved: the overshoot past
    /// the edge becomes the distance from the edge on the neighbouring sector.
    pub fn wrapped(self) -> SurfacePoint {
        let mut point = self;
        // Two folds cover a corner crossing; more means an absurd step.
        for _ in 0..4 {
            let (a, b) = (point.u - HALF, point.v - HALF);
            let (du, dv, over) = if a > HALF {
                (1.0, 0.0, a - HALF)
            } else if a < -HALF {
                (-1.0, 0.0, -HALF - a)
            } else if b > HALF {
                (0.0, 1.0, b - HALF)
            } else if b < -HALF {
                (0.0, -1.0, -HALF - b)
            } else {
                return point;
            };
            let [n, ua, va] = frame(point.sector);
            // Travel direction over the edge, and the in-face coordinate that
            // runs along the edge and is kept as is.
            let t = vec3::add(vec3::scale(ua, du), vec3::scale(va, dv));
            let along = vec3::add(vec3::scale(ua, a * dv.abs()), vec3::scale(va, b * du.abs()));
            let p = vec3::add(
                vec3::add(vec3::scale(n, HALF - over), vec3::scale(t, HALF)),
                along,
            );
            let sector = Sector::containing(t);
            let [_, ub, vb] = frame(sector);
            point = SurfacePoint {
                sector,
                u: vec3::dot(p, ub) + HALF,
                v: vec3::dot(p, vb) + HALF,
            };
        }
        panic!("surface point too far out of range to wrap: {self:?}");
    }
}

/// `[n, u, v]` of a sector as float vectors.
pub(crate) fn frame(sector: Sector) -> [[f64; 3]; 3] {
    [sector.normal(), sector.u_axis(), sector.v_axis()].map(|axis| axis.map(f64::from))
}
