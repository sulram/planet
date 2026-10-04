//! A platform: what a build stands on, built.
//!
//! A slab one cell thick with its top at one height, on a base down to the
//! ground: a deck's pillars, every column filled, or nothing at all, a slab
//! that floats. Nothing here knows what the ground is: whoever seats the
//! volumes says how high the slab stands and how high the ground is under
//! each column, in cells, and the base is as tall as the drop under it.
//!
//! A platform is cut by the frame, as a plot is: those of one size tile it,
//! and two side by side meet edge to edge with their pillars paired.

use voxel::{Gesture, Span};

/// What carries a slab down to the ground.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Base {
    /// A deck: a pillar at the corners of every bay, and open under the slab.
    #[default]
    Deck,
    /// Every column filled down to the ground: a block standing on it.
    Solid,
    /// Nothing: the slab alone, floating where it is laid.
    Floating,
}

/// A square of columns under a slab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Platform {
    /// The first column, `x` and `y`.
    pub corner: [i32; 2],
    /// Columns along a side.
    pub side: u32,
    /// The height of the top of the slab.
    pub top: i32,
}

impl Platform {
    /// Columns from one row of pillars to the next, at most.
    pub const BAY: u32 = 16;

    /// The platform of `2^bits` columns a side that a column is on.
    pub fn over(x: i32, y: i32, bits: u32, top: i32) -> Platform {
        Platform {
            corner: [x >> bits << bits, y >> bits << bits],
            side: 1 << bits,
            top,
        }
    }

    /// The box of the slab.
    pub fn slab(&self) -> Span {
        let [x, y] = self.corner;
        let far = self.side as i32 - 1;
        Span::between([x, y, self.top - 1], [x + far, y + far, self.top - 1])
    }

    /// Whether a column carries a pillar: the corners of every bay.
    fn pillar(&self, x: i32, y: i32) -> bool {
        let bay = self.side.min(Platform::BAY) as i32;
        let corner = |at: i32, from: i32| {
            let along = (at - from).rem_euclid(bay);
            along == 0 || along == bay - 1
        };
        corner(x, self.corner[0]) && corner(y, self.corner[1])
    }

    /// The gestures that build it in one paint, over ground `ground(x, y)`
    /// cells high: the slab, then its base from the ground to the slab
    /// wherever there is a drop to span. A solid base is one box for each run
    /// of columns along a row whose ground stands as high, so a slope is a
    /// staircase of a few boxes and level ground one.
    pub fn gestures(
        &self,
        base: Base,
        ground: impl Fn(i32, i32) -> i32,
        paint: u8,
    ) -> Vec<Gesture> {
        let slab = self.slab();
        let z = self.top - 1;
        let mut gestures = vec![Gesture::Create { span: slab, paint }];
        for y in slab.min[1]..=slab.max[1] {
            let mut x = slab.min[0];
            while x <= slab.max[0] {
                let foot = ground(x, y);
                let mut end = x;
                let carried = match base {
                    Base::Deck => self.pillar(x, y),
                    Base::Solid => {
                        while end < slab.max[0] && ground(end + 1, y) == foot {
                            end += 1;
                        }
                        true
                    }
                    Base::Floating => false,
                };
                if carried && foot < z {
                    gestures.push(Gesture::Create {
                        span: Span::between([x, y, foot], [end, y, z - 1]),
                        paint,
                    });
                }
                x = end + 1;
            }
        }
        gestures
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel::Volumes;

    fn built(platform: Platform, ground: impl Fn(i32, i32) -> i32) -> Volumes {
        built_on(Base::Deck, platform, ground)
    }

    fn built_on(base: Base, platform: Platform, ground: impl Fn(i32, i32) -> i32) -> Volumes {
        let mut volumes = Volumes::new(6);
        for plot in [[0, 0], [1, 0]] {
            volumes.open(plot, 0, 64);
        }
        for gesture in platform.gestures(base, ground, 1) {
            volumes.apply(gesture);
        }
        volumes
    }

    #[test]
    fn a_platform_is_cut_by_the_frame() {
        let platform = Platform::over(37, 70, 4, 12);
        assert_eq!(platform.corner, [32, 64]);
        assert_eq!(platform.side, 16);
        assert_eq!(platform.slab(), Span::between([32, 64, 11], [47, 79, 11]));
        assert_eq!(Platform::over(-3, 9, 3, 0).corner, [-8, 8]);
        // Two of one size, a column apart across their edge, tile.
        let beside = Platform::over(48, 70, 4, 12);
        assert_eq!(beside.slab().min[0], platform.slab().max[0] + 1);
    }

    #[test]
    fn on_level_ground_it_is_a_slab_and_nothing_under_it() {
        let platform = Platform::over(20, 20, 4, 10);
        let volumes = built(platform, |_, _| 9);
        for at in platform.slab().cells() {
            assert_eq!(volumes.get(at).paint(), Some(1), "{at:?}");
        }
        let under = platform.slab().moved([0, 0, -1]);
        assert!(under.cells().all(|at| volumes.get(at).is_air()));
        assert!(volumes.get([15, 20, 9]).is_air());
    }

    #[test]
    fn over_a_drop_it_stands_on_pillars_down_to_the_ground() {
        // The ground falls a cell every two columns along `x`.
        let ground = |x: i32, _: i32| 9 - (x - 16) / 2;
        let platform = Platform::over(20, 20, 4, 10);
        let volumes = built(platform, ground);
        for (x, y) in [(16, 16), (31, 16), (16, 31), (31, 31)] {
            let foot = ground(x, y);
            for z in foot..9 {
                assert_eq!(volumes.get([x, y, z]).paint(), Some(1), "{x} {y} {z}");
            }
            assert!(volumes.get([x, y, foot - 1]).is_air());
        }
        // Between the pillars it is open under the slab.
        assert!(volumes.get([24, 24, 5]).is_air());
        assert_eq!(volumes.get([24, 24, 9]).paint(), Some(1));
    }

    #[test]
    fn a_solid_base_fills_every_column_down_to_the_ground() {
        // The ground falls a cell every two columns along `x`, and rises a
        // cell every three along `y`.
        let ground = |x: i32, y: i32| 9 - (x - 16) / 2 + (y - 16) / 3;
        let platform = Platform::over(20, 20, 4, 18);
        let volumes = built_on(Base::Solid, platform, ground);
        for y in 16..32 {
            for x in 16..32 {
                let foot = ground(x, y);
                for z in foot..18 {
                    assert_eq!(volumes.get([x, y, z]).paint(), Some(1), "{x} {y} {z}");
                }
                assert!(volumes.get([x, y, foot - 1]).is_air(), "{x} {y}");
            }
        }
        // Around it there is nothing.
        assert!(volumes.get([15, 20, 8]).is_air());
        assert!(volumes.get([32, 20, 8]).is_air());
    }

    #[test]
    fn a_solid_base_is_a_box_for_each_run_of_level_ground() {
        let platform = Platform::over(0, 0, 4, 10);
        // Level: the slab and one box a row.
        let level = platform.gestures(Base::Solid, |_, _| 4, 1);
        assert_eq!(level.len(), 1 + 16);
        // A step down half way along every row: two boxes a row.
        let step = platform.gestures(Base::Solid, |x, _| if x < 8 { 4 } else { 3 }, 1);
        assert_eq!(step.len(), 1 + 2 * 16);
        // Ground as high as the slab needs nothing under it.
        let flush = platform.gestures(Base::Solid, |_, _| 9, 1);
        assert_eq!(flush.len(), 1);
    }

    #[test]
    fn a_wide_platform_has_a_pillar_at_every_bay() {
        let platform = Platform::over(0, 0, 6, 10);
        let volumes = built(platform, |_, _| 0);
        let pillars: Vec<i32> = (0..64)
            .filter(|&x| !volumes.get([x, 0, 4]).is_air())
            .collect();
        assert_eq!(pillars, vec![0, 15, 16, 31, 32, 47, 48, 63]);
        // A narrow one stands on its own four corners.
        let narrow = Platform::over(64, 0, 3, 10);
        let volumes = built(narrow, |_, _| 0);
        let pillars: Vec<i32> = (64..72)
            .filter(|&x| !volumes.get([x, 0, 4]).is_air())
            .collect();
        assert_eq!(pillars, vec![64, 71]);
    }

    #[test]
    fn a_floating_platform_has_nothing_under_its_slab() {
        let platform = Platform::over(0, 0, 4, 20);
        let gestures = platform.gestures(Base::Floating, |x, _| x / 2, 3);
        assert_eq!(
            gestures,
            [Gesture::Create {
                span: platform.slab(),
                paint: 3
            }]
        );
        // The same slab on a deck stands on pillars down to the ground.
        assert!(platform.gestures(Base::Deck, |x, _| x / 2, 3).len() > 1);
    }
}
