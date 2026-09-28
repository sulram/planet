//! Stamps: ground held flat under something that is not terrain.
//!
//! A stamp is how anything that is not terrain seats into terrain (DECISIONS
//! 58): a footprint in address space where the ground stands at one height,
//! and a margin around it where the ground eases back to its own. It is
//! applied when the ground is sampled, at every footprint, so the heightfield,
//! collision, spawning and grass all see the same ground, and none of it is an
//! edit.

use topology::{QuadSphere, Sector, SurfacePoint, vec3};

use crate::Direction;
use crate::noise::smoothstep;

/// Flat ground over a box of columns of one sector.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Stamp {
    sector: Sector,
    /// The footprint, in blocks: `u[0]..u[1]` by `v[0]..v[1]`.
    u: [f64; 2],
    v: [f64; 2],
    height_m: f64,
    /// Metres over which the ground outside eases back to its own height.
    blend_m: f64,
    /// Metres per block along `u` and `v` at the middle of the footprint:
    /// what turns a distance in blocks into the margin's metres.
    block_m: [f64; 2],
    /// A cap around the footprint and its margin, for the test every sample
    /// pays: its middle, and the cosine of the widest angle it reaches.
    middle: Direction,
    reach_cos: f64,
}

impl Stamp {
    /// The ground over columns `u[0]..u[1]` by `v[0]..v[1]` of a sector held
    /// at `height_m` over the datum, easing back over `blend_m` around them.
    pub fn new(
        sphere: QuadSphere,
        sector: Sector,
        u: [f64; 2],
        v: [f64; 2],
        height_m: f64,
        blend_m: f64,
    ) -> Stamp {
        let grid = sphere.blocks();
        let middle_point = SurfacePoint::new(sector, (u[0] + u[1]) / 2.0, (v[0] + v[1]) / 2.0);
        let tangents = grid.tangents(middle_point);
        let radius_m = sphere.radius_m();
        let block_m = [
            vec3::length(tangents.du) * radius_m,
            vec3::length(tangents.dv) * radius_m,
        ];
        // The whole diagonal is twice what reaches a corner from the middle,
        // and blocks shrink by at most a square root of two toward a sector
        // corner, so it bounds the footprint wherever it is.
        let diagonal_m = libm::sqrt(
            (u[1] - u[0]) * (u[1] - u[0]) * block_m[0] * block_m[0]
                + (v[1] - v[0]) * (v[1] - v[0]) * block_m[1] * block_m[1],
        );
        let reach = ((diagonal_m + 2.0 * blend_m) / radius_m).min(core::f64::consts::PI);
        Stamp {
            sector,
            u,
            v,
            height_m,
            blend_m,
            block_m,
            middle: tangents.up,
            reach_cos: libm::cos(reach),
        }
    }

    pub fn height_m(&self) -> f64 {
        self.height_m
    }

    /// The middle of the footprint, and the angle from it that the stamp
    /// and its margin reach: what a streamer asks to know which ground to
    /// draw again.
    pub fn cap(&self) -> (Direction, f64) {
        (self.middle, libm::acos(self.reach_cos))
    }

    /// Whether a direction is close enough to be touched at all. One dot
    /// product: this is what every sample of the whole world pays.
    pub(crate) fn near(&self, direction: Direction) -> bool {
        vec3::dot(direction, self.middle) >= self.reach_cos
    }

    /// How much of the stamp's height a point takes: all of it on the
    /// footprint, none past the margin, a smooth step between.
    pub(crate) fn weight(&self, point: SurfacePoint) -> f64 {
        if point.sector != self.sector {
            return 0.0;
        }
        let out = |x: f64, [lo, hi]: [f64; 2]| (lo - x).max(x - hi).max(0.0);
        let du = out(point.u, self.u) * self.block_m[0];
        let dv = out(point.v, self.v) * self.block_m[1];
        let outside_m = libm::sqrt(du * du + dv * dv);
        1.0 - smoothstep(0.0, self.blend_m, outside_m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_takes_all_and_past_the_margin_takes_nothing() {
        let sphere = QuadSphere::new(16).unwrap();
        let sector = Sector::new(0).unwrap();
        let stamp = Stamp::new(
            sphere,
            sector,
            [1000.0, 1064.0],
            [2000.0, 2064.0],
            12.0,
            8.0,
        );
        let at = |u: f64, v: f64| stamp.weight(SurfacePoint::new(sector, u, v));
        assert_eq!(at(1030.0, 2030.0), 1.0);
        assert_eq!(at(1064.0, 2000.0), 1.0);
        assert!((0.0..1.0).contains(&at(1070.0, 2030.0)));
        assert_eq!(at(1100.0, 2030.0), 0.0);
        let elsewhere = SurfacePoint::new(Sector::new(1).unwrap(), 1030.0, 2030.0);
        assert_eq!(stamp.weight(elsewhere), 0.0);
    }

    #[test]
    fn a_stamped_generator_holds_the_ground_and_leaves_the_rest() {
        let mut generator = crate::Generator::new(crate::Recipe::new(7)).unwrap();
        let grid = generator.sphere().blocks();
        let sector = Sector::new(2).unwrap();
        let far = grid.direction(SurfacePoint::new(sector, 9000.0, 9000.0));
        let before = generator.sample(far).height_m;
        generator.stamp(Stamp::new(
            generator.sphere(),
            sector,
            [4000.0, 4064.0],
            [4000.0, 4064.0],
            37.5,
            10.0,
        ));
        let inside = grid.direction(SurfacePoint::new(sector, 4010.0, 4050.0));
        for footprint_m in [0.0, 4.0, 64.0] {
            assert_eq!(generator.sample_at(inside, footprint_m).height_m, 37.5);
        }
        assert_eq!(generator.sample(far).height_m, before);
    }

    #[test]
    fn the_cap_holds_the_margin() {
        let sphere = QuadSphere::new(16).unwrap();
        let grid = sphere.blocks();
        let sector = Sector::new(3).unwrap();
        let stamp = Stamp::new(sphere, sector, [500.0, 564.0], [700.0, 764.0], 0.0, 12.0);
        for (u, v) in [(470.0, 700.0), (600.0, 800.0), (532.0, 732.0)] {
            let point = SurfacePoint::new(sector, u, v);
            if stamp.weight(point) > 0.0 {
                assert!(stamp.near(grid.direction(point)), "{u} {v}");
            }
        }
    }
}
