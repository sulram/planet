//! A part: a placed thing that is no cell (DECISIONS 111). A cube first; a
//! model, a picture, a light are shapes of the same thing.
//!
//! A part knows no sphere (CLAUDE.md, How it grows). It stands in a flat
//! frame, blocks along `x` and `y` and up `z`, the frame of the seat it is
//! on, and seating it on a body is `seat`'s job: the frame is the seat's
//! tangent frame at the anchor, and a part is rigid in it. Everything here
//! is said in that frame and in blocks: where a line meets a part, what a
//! body standing beside one has under its feet and over its head.
//!
//! A part's own space is the unit box: `x` and `y` from `-0.5` to `0.5`,
//! `z` from `0` to `1`, its anchor the middle of its bottom. Its pose turns
//! and stretches that box, so a cube of scale 1 is one cell, and a model's
//! unit is one of its own voxels.

use glam::{DAffine3, DQuat, DVec3};

/// The name of a part, as the world gives it: never reused.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct PartId(pub u64);

/// What a part is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shape {
    /// The unit box, in one paint.
    Cube,
}

/// Where a part stands and how it is turned and stretched, in the flat
/// frame of its seat.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    /// Where its anchor stands: blocks along the seat's `u` and `v`, and
    /// blocks over the datum.
    pub at: [f64; 3],
    /// How it is turned, in the frame of where it stands: `x` along `u`, `y`
    /// along `v`, `z` up.
    pub rotation: DQuat,
    /// How long its unit box is along each of its own axes, in blocks.
    pub scale: [f64; 3],
}

impl Pose {
    /// Standing upright at a point, as big as `scale` says.
    pub fn upright(at: [f64; 3], scale: [f64; 3]) -> Pose {
        Pose {
            at,
            rotation: DQuat::IDENTITY,
            scale,
        }
    }

    /// The flat frame from the part's own space, around the anchor: how its
    /// unit box is turned and stretched. The anchor itself is not in it, so
    /// a seat adds where the anchor is.
    pub fn local(&self) -> DAffine3 {
        DAffine3::from_rotation_translation(self.rotation, DVec3::ZERO)
            * DAffine3::from_scale(DVec3::from(self.scale))
    }

    /// The part's own space from the flat frame, with the anchor in it: what
    /// a point of the seat is in the unit box.
    pub fn inward(&self) -> DAffine3 {
        self.local().inverse() * DAffine3::from_translation(-DVec3::from(self.at))
    }

    /// Whether the pose is sound: finite, a unit rotation, every length
    /// positive. What the world refuses otherwise.
    pub fn is_sound(&self) -> bool {
        let finite = self.at.iter().chain(&self.scale).all(|n| n.is_finite());
        finite
            && self.rotation.is_finite()
            && self.rotation.is_normalized()
            && self.scale.iter().all(|&n| n >= MIN_SCALE)
    }
}

/// The least a part is along any axis, in blocks: thin as a pane, never
/// nothing.
pub const MIN_SCALE: f64 = 0.01;

/// A part as it is kept and told.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Part {
    pub shape: Shape,
    pub pose: Pose,
    /// What it is made of: a paint of the palette, as a cell's (`voxel`).
    pub paint: u8,
    /// Whether a body stands on it and is stopped by it.
    pub solid: bool,
}

/// What holds up a body standing beside a part: the top of it under the
/// feet and the bottom of it over the head, in blocks over the datum.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Gap {
    pub floor: Option<f64>,
    pub ceiling: Option<f64>,
}

impl Part {
    /// How far along a line from `from` toward `toward`, unit, both in the
    /// flat frame, the line goes into this part. `None` when it misses, or
    /// starts inside.
    pub fn hit(&self, from: DVec3, toward: DVec3) -> Option<f64> {
        let inward = self.pose.inward();
        let (from, toward) = (
            inward.transform_point3(from),
            inward.transform_vector3(toward),
        );
        let (near, far) = slab(from, toward)?;
        (near > 0.0 && far >= near).then_some(near)
    }

    /// What a body standing in a column has under its feet and over its
    /// head of this part, when its step reaches `reach` blocks over the
    /// datum: as the cells answer it (`voxel::Volumes::gap`). The body is
    /// `half` blocks wide each way of `column`. `None` where the part is
    /// not solid, or not over the body at all.
    ///
    /// Solid at the height of the step is a wall, and the wall's top is the
    /// floor it offers, which a walker refuses when it is more than a step
    /// up. Air there means the part below is the floor and the part above
    /// is the roof.
    pub fn gap(&self, column: [f64; 2], half: f64, reach: f64) -> Option<Gap> {
        if !self.solid {
            return None;
        }
        let inward = self.pose.inward();
        let up = inward.transform_vector3(DVec3::Z);
        let feet = [
            [0.0, 0.0],
            [-half, -half],
            [half, -half],
            [-half, half],
            [half, half],
        ];
        let mut gap: Option<Gap> = None;
        for [dx, dy] in feet {
            let foot = inward.transform_point3(DVec3::new(column[0] + dx, column[1] + dy, 0.0));
            let Some((low, high)) = slab(foot, up) else {
                continue;
            };
            let found = if low <= reach && reach <= high || high <= reach {
                Gap {
                    floor: Some(high),
                    ceiling: None,
                }
            } else {
                Gap {
                    floor: None,
                    ceiling: Some(low),
                }
            };
            gap = Some(match gap {
                None => found,
                Some(gap) => Gap {
                    floor: pick(gap.floor, found.floor, f64::max),
                    ceiling: pick(gap.ceiling, found.ceiling, f64::min),
                },
            });
        }
        gap
    }

    /// The flat frame from the part's own space, with the anchor in it:
    /// where each corner of its unit box stands on the seat.
    pub fn placed(&self) -> DAffine3 {
        DAffine3::from_translation(DVec3::from(self.pose.at)) * self.pose.local()
    }
}

/// Where a line `from + toward * t`, said in the space of a unit box, is
/// inside it: the `t` it goes in and the `t` it comes out, or `None` when it
/// never is. What a handle drawn as a box is picked by.
pub fn through(from: DVec3, toward: DVec3) -> Option<(f64, f64)> {
    slab(from, toward)
}

fn pick(a: Option<f64>, b: Option<f64>, f: fn(f64, f64) -> f64) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(f(a, b)),
        (a, b) => a.or(b),
    }
}

/// Where a line `from + toward * t` is inside the unit box: the `t` it goes
/// in and the `t` it comes out, or `None` when it never is. A line along a
/// side counts as inside it.
fn slab(from: DVec3, toward: DVec3) -> Option<(f64, f64)> {
    let (low, high) = (DVec3::new(-0.5, -0.5, 0.0), DVec3::new(0.5, 0.5, 1.0));
    let (mut near, mut far) = (f64::NEG_INFINITY, f64::INFINITY);
    for axis in 0..3 {
        let (d, f) = (toward[axis], from[axis]);
        if d.abs() < 1e-12 {
            if f < low[axis] || f > high[axis] {
                return None;
            }
            continue;
        }
        let (a, b) = ((low[axis] - f) / d, (high[axis] - f) / d);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far).then_some((near, far))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::FRAC_PI_4;
    use proptest::prelude::*;

    fn cube(pose: Pose) -> Part {
        Part {
            shape: Shape::Cube,
            pose,
            paint: 0,
            solid: true,
        }
    }

    #[test]
    fn a_line_meets_a_cube_where_its_side_is() {
        let part = cube(Pose::upright([10.0, 10.0, 4.0], [2.0, 2.0, 2.0]));
        // From the side, along x: the near side is a block before the anchor.
        let hit = part.hit(DVec3::new(0.0, 10.0, 5.0), DVec3::X).unwrap();
        assert!((hit - 9.0).abs() < 1e-9, "{hit}");
        // From above: the top is two blocks over the anchor.
        let hit = part.hit(DVec3::new(10.0, 10.0, 20.0), -DVec3::Z).unwrap();
        assert!((hit - 14.0).abs() < 1e-9, "{hit}");
        // Past it.
        assert!(part.hit(DVec3::new(0.0, 12.0, 5.0), DVec3::X).is_none());
        // From inside: nothing is met.
        assert!(part.hit(DVec3::new(10.0, 10.0, 5.0), DVec3::X).is_none());
    }

    #[test]
    fn a_body_stands_on_a_cube_and_under_one() {
        let part = cube(Pose::upright([10.0, 10.0, 4.0], [2.0, 2.0, 2.0]));
        // Feet at the top: the floor is the top.
        let gap = part.gap([10.0, 10.0], 0.6, 7.0).unwrap();
        assert_eq!(
            gap,
            Gap {
                floor: Some(6.0),
                ceiling: None
            }
        );
        // Feet on the ground beside it, a step reaching into it: a wall,
        // whose top is the floor it offers.
        let gap = part.gap([10.0, 10.0], 0.6, 5.0).unwrap();
        assert_eq!(gap.floor, Some(6.0));
        // Feet well under it: a roof.
        let gap = part.gap([10.0, 10.0], 0.6, 1.0).unwrap();
        assert_eq!(
            gap,
            Gap {
                floor: None,
                ceiling: Some(4.0)
            }
        );
        // Beside it, out of reach of the body's width: nothing.
        assert!(part.gap([13.0, 10.0], 0.6, 5.0).is_none());
        // Within the body's width: the edge holds it.
        assert!(part.gap([11.4, 10.0], 0.6, 7.0).is_some());
        // Not solid: nothing, however near.
        let open = Part {
            solid: false,
            ..part
        };
        assert!(open.gap([10.0, 10.0], 0.6, 7.0).is_none());
    }

    #[test]
    fn a_turned_cube_is_met_on_its_turned_side() {
        // Turned an eighth about z, a cube of 2 reaches sqrt(2) from its
        // anchor along x.
        let pose = Pose {
            at: [0.0, 0.0, 0.0],
            rotation: DQuat::from_rotation_z(FRAC_PI_4),
            scale: [2.0, 2.0, 2.0],
        };
        let hit = cube(pose)
            .hit(DVec3::new(-10.0, 0.0, 1.0), DVec3::X)
            .unwrap();
        assert!((hit - (10.0 - 2f64.sqrt())).abs() < 1e-9, "{hit}");
        // Turned onto its side about y, its top is now where its side was.
        let pose = Pose {
            at: [0.0, 0.0, 0.0],
            rotation: DQuat::from_rotation_y(core::f64::consts::FRAC_PI_2),
            scale: [2.0, 2.0, 4.0],
        };
        let gap = cube(pose).gap([0.0, 0.0], 0.0, 10.0).unwrap();
        assert!((gap.floor.unwrap() - 1.0).abs() < 1e-9, "{gap:?}");
    }

    proptest! {
        #[test]
        fn what_a_line_meets_is_on_the_box(
            at in prop::array::uniform3(-50.0f64..50.0),
            axis in prop::array::uniform3(-1.0f64..1.0),
            angle in 0.0f64..6.3,
            scale in prop::array::uniform3(0.1f64..10.0),
            from in prop::array::uniform3(-100.0f64..100.0),
        ) {
            let axis = DVec3::from(axis);
            prop_assume!(axis.length() > 1e-3);
            let pose = Pose { at, rotation: DQuat::from_axis_angle(axis.normalize(), angle), scale };
            let part = cube(pose);
            let from = DVec3::from(from);
            let toward = (DVec3::from(at) - from).normalize();
            // Aimed at the anchor from outside, the line meets the box, and
            // where it does is on a side of the unit box.
            let hit = part.hit(from, toward);
            prop_assume!(hit.is_some());
            let on = pose.inward().transform_point3(from + toward * hit.unwrap());
            let side = [on.x.abs() - 0.5, on.y.abs() - 0.5, (on.z - 0.5).abs() - 0.5];
            prop_assert!(side.iter().any(|d| d.abs() < 1e-6), "{on:?}");
            prop_assert!(side.iter().all(|d| *d < 1e-6), "{on:?}");
        }
    }
}
