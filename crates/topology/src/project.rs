//! Address space <-> world space.
//!
//! The mapping is the tangent warp: a grid coordinate `s` in `-1..=1` moves to
//! `tan(s * pi / 4)` on the cube face before the face is pushed onto the
//! sphere. Grid lines become equally spaced great circles through the face
//! axes, which keeps blocks near square: a block edge is `BLOCK_M` at a sector
//! centre and shrinks to `BLOCK_M / sqrt(2)` at the corners, never worse
//! (checked by `block_distortion_is_bounded`).

use core::f64::consts::FRAC_PI_4;

use crate::address::Address;
use crate::sector::Sector;
use crate::surface::{SurfacePoint, frame};
use crate::vec3::{self, Vec3};
use crate::{BLOCK_M, RADIUS_M, SECTOR_SIDE};

const HALF: f64 = SECTOR_SIDE as f64 / 2.0;

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

impl SurfacePoint {
    /// The unit vector from the planet centre through this point.
    pub fn direction(self) -> Vec3 {
        let [n, ua, va] = frame(self.sector);
        let (x, y) = (warp(self.u), warp(self.v));
        vec3::normalize(vec3::add(
            n,
            vec3::add(vec3::scale(ua, x), vec3::scale(va, y)),
        ))
    }

    /// The surface point a direction passes through. `d` need not be unit.
    pub fn from_direction(d: Vec3) -> SurfacePoint {
        let sector = Sector::containing(d);
        let [n, ua, va] = frame(sector);
        let depth = vec3::dot(d, n);
        SurfacePoint {
            sector,
            u: unwarp(vec3::dot(d, ua) / depth),
            v: unwarp(vec3::dot(d, va) / depth),
        }
    }

    /// World position at `height_m` metres above the datum sphere.
    pub fn position(self, height_m: f64) -> Vec3 {
        vec3::scale(self.direction(), RADIUS_M + height_m)
    }

    /// Local up and the partial derivatives of [`SurfacePoint::direction`].
    ///
    /// `du` and `dv` are tangent to the sphere but neither unit nor exactly
    /// orthogonal: they are the honest image of the address space axes.
    pub fn tangents(self) -> Tangents {
        let [n, ua, va] = frame(self.sector);
        let (x, y) = (warp(self.u), warp(self.v));
        let q = vec3::add(n, vec3::add(vec3::scale(ua, x), vec3::scale(va, y)));
        let len = vec3::length(q);
        let up = vec3::scale(q, 1.0 / len);
        // d/ds tan(s * pi/4) = pi/4 * (1 + tan^2), and ds = 1 / HALF per block.
        let partial = |axis: Vec3, w: f64| {
            let dq = vec3::scale(axis, FRAC_PI_4 * (1.0 + w * w) / HALF);
            vec3::scale(vec3::sub(dq, vec3::scale(up, vec3::dot(up, dq))), 1.0 / len)
        };
        Tangents {
            du: partial(ua, x),
            dv: partial(va, y),
            up,
        }
    }
}

impl Address {
    /// World position of the centre of the block.
    pub fn position(self) -> Vec3 {
        SurfacePoint::center_of(self.column).position((f64::from(self.h) + 0.5) * BLOCK_M)
    }
}

/// Block coordinate `0..=SECTOR_SIDE` to cube face coordinate `-1..=1`, warped.
fn warp(blocks: f64) -> f64 {
    libm::tan((blocks - HALF) / HALF * FRAC_PI_4)
}

fn unwarp(face: f64) -> f64 {
    libm::atan(face) / FRAC_PI_4 * HALF + HALF
}
