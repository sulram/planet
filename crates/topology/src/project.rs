//! Address space <-> world space.
//!
//! The mapping is the tangent warp: a grid coordinate `s` in `-1..=1` moves to
//! `tan(s * pi / 4)` on the cube face before the face is pushed onto the
//! sphere. Grid lines become equally spaced great circles through the face
//! axes, which keeps blocks near square: a block edge is `BLOCK_M` at a sector
//! centre and shrinks to `BLOCK_M / sqrt(2)` at the corners, never worse
//! (checked by `block_distortion_is_bounded`).
//!
//! The warp is the same shape at every size; only the span of the grid it is
//! fed changes, so a small body is not a distorted one.

use core::f64::consts::FRAC_PI_4;

use crate::address::Address;
use crate::grid::Grid;
use crate::quad_sphere::QuadSphere;
use crate::sector::Sector;
use crate::surface::{SurfacePoint, frame};
use crate::vec3::{self, Vec3};
use crate::{BLOCK_M, Column};

/// How the unit sphere moves when a surface point moves in address space.
#[derive(Clone, Copy, Debug)]
pub struct Tangents {
    /// Change of the unit direction per block along `u`.
    pub du: Vec3,
    /// Change of the unit direction per block along `v`.
    pub dv: Vec3,
    /// The unit direction itself: local up.
    pub up: Vec3,
}

impl Grid {
    /// The unit vector from the body's centre through a surface point.
    pub fn direction(self, point: SurfacePoint) -> Vec3 {
        let [n, ua, va] = frame(point.sector);
        let (x, y) = (self.warp(point.u), self.warp(point.v));
        vec3::normalize(vec3::add(
            n,
            vec3::add(vec3::scale(ua, x), vec3::scale(va, y)),
        ))
    }

    /// The surface point a direction passes through. `d` need not be unit.
    pub fn surface_point(self, d: Vec3) -> SurfacePoint {
        let sector = Sector::containing(d);
        let [n, ua, va] = frame(sector);
        let depth = vec3::dot(d, n);
        SurfacePoint {
            sector,
            u: self.unwarp(vec3::dot(d, ua) / depth),
            v: self.unwarp(vec3::dot(d, va) / depth),
        }
    }

    /// The unit direction through the centre of a column.
    pub fn column_direction(self, column: Column) -> Vec3 {
        self.direction(SurfacePoint::center_of(column))
    }

    /// Local up and the partial derivatives of [`Grid::direction`].
    ///
    /// `du` and `dv` are tangent to the sphere but neither unit nor exactly
    /// orthogonal: they are the honest image of the address space axes.
    pub fn tangents(self, point: SurfacePoint) -> Tangents {
        let [n, ua, va] = frame(point.sector);
        let (x, y) = (self.warp(point.u), self.warp(point.v));
        let q = vec3::add(n, vec3::add(vec3::scale(ua, x), vec3::scale(va, y)));
        let len = vec3::length(q);
        let up = vec3::scale(q, 1.0 / len);
        // d/ds tan(s * pi/4) = pi/4 * (1 + tan^2), and ds = 1 / half per block.
        let half = self.half();
        let partial = |axis: Vec3, w: f64| {
            let dq = vec3::scale(axis, FRAC_PI_4 * (1.0 + w * w) / half);
            vec3::scale(vec3::sub(dq, vec3::scale(up, vec3::dot(up, dq))), 1.0 / len)
        };
        Tangents {
            du: partial(ua, x),
            dv: partial(va, y),
            up,
        }
    }

    /// Block coordinate `0..=side` to cube face coordinate `-1..=1`, warped.
    fn warp(self, blocks: f64) -> f64 {
        let half = self.half();
        libm::tan((blocks - half) / half * FRAC_PI_4)
    }

    fn unwarp(self, face: f64) -> f64 {
        let half = self.half();
        libm::atan(face) / FRAC_PI_4 * half + half
    }
}

impl QuadSphere {
    /// World position at `height_m` metres above the datum sphere.
    pub fn position(self, point: SurfacePoint, height_m: f64) -> Vec3 {
        let direction = self.blocks().direction(point);
        vec3::scale(direction, self.radius_m() + height_m)
    }

    /// World position of the centre of a block.
    pub fn block_centre(self, address: Address) -> Vec3 {
        let point = SurfacePoint::center_of(address.column);
        self.position(point, (f64::from(address.h) + 0.5) * BLOCK_M)
    }
}
