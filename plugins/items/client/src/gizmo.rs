//! The gizmo: the handles over the part picked, one along each of its own
//! axes, and a drag of one that moves, turns or stretches the part. A drag
//! changes the part's world transform and asks the host what pose that is
//! (`Host::reseat`), so a part dragged over a seam is the same part on the
//! next sector.

use client::{Host, Mark, Part, PartId, Seat};
use glam::{DAffine3, DMat3, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use topology::BLOCK_M;
use voxel::{Finish, Paint};

/// What a drag of a handle does to the part.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Move,
    Rotate,
    Scale,
}

/// How far a handle reaches from the part's centre, metres, and how thick
/// its bar is.
const REACH_M: f64 = 1.0;
const BAR_M: f64 = 0.04;
/// The tip at the end of a bar: a cube to move by, a tile to turn by, a
/// larger cube to stretch by, metres.
const TIP_M: f64 = 0.14;
const TILE_M: f64 = 0.36;
/// The colours of the three axes, and of the one under the pointer or
/// being dragged: red, green, blue and yellow of the palette, as lights, so
/// they read over anything.
const AXIS_PAINTS: [u8; 3] = [4, 9, 11];
const HOT_PAINT: u8 = 15;

/// The part picked and the handles over it.
pub(crate) struct Gizmo {
    pub id: PartId,
    pub seat: Seat,
    pub part: Part,
    /// The world from the part's own space, from the planet's centre, as
    /// it stands now.
    pub world: DAffine3,
    /// The axis whose handle is under the pointer.
    pub hot: Option<usize>,
    pub drag: Option<Drag>,
}

/// A handle being dragged: which axis, where the drag started along it and
/// the part as it stood then.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Drag {
    pub axis: usize,
    pub start: f64,
    pub from: DAffine3,
}

/// A handle as a box in the world: the world from a unit box.
fn handle(world: &DAffine3, axis: usize, mode: Mode, tip: bool) -> DAffine3 {
    let axes = [world.x_axis, world.y_axis, world.z_axis].map(|a| a.normalize());
    let centre = world.transform_point3(DVec3::new(0.0, 0.0, 0.5));
    let (u, v, w) = (axes[(axis + 1) % 3], axes[(axis + 2) % 3], axes[axis]);
    let frame = DMat3::from_cols(u, v, w);
    if tip {
        let size = match mode {
            Mode::Move => DVec3::splat(TIP_M),
            Mode::Rotate => DVec3::new(TILE_M, BAR_M, TIP_M),
            Mode::Scale => DVec3::splat(TIP_M * 1.5),
        };
        DAffine3::from_mat3_translation(frame, centre + w * REACH_M) * DAffine3::from_scale(size)
    } else {
        DAffine3::from_mat3_translation(frame, centre)
            * DAffine3::from_scale(DVec3::new(BAR_M, BAR_M, REACH_M))
    }
}

impl Gizmo {
    pub fn new(host: &Host<'_>, id: PartId) -> Option<Gizmo> {
        let (seat, part) = host.parts().get(id)?;
        let world = host.part_transform(id)?;
        Some(Gizmo {
            id,
            seat,
            part: *part,
            world,
            hot: None,
            drag: None,
        })
    }

    /// Reads the part again: it may have been changed by the panel, or by
    /// the world.
    pub fn refresh(&mut self, host: &Host<'_>) -> bool {
        match (host.parts().get(self.id), host.part_transform(self.id)) {
            (Some((seat, part)), Some(world)) => {
                self.seat = seat;
                self.part = *part;
                self.world = world;
                true
            }
            _ => false,
        }
    }

    /// The handles as marks over the world: a bar and a tip an axis, the
    /// hot one in the hot colour.
    pub fn marks(&self, mode: Mode) -> Vec<Mark> {
        let paint = |axis: usize| {
            let color = match self.hot == Some(axis) {
                true => HOT_PAINT,
                false => AXIS_PAINTS[axis],
            };
            Paint {
                color,
                finish: Finish::Light,
                edge: voxel::Edge::None,
            }
            .byte()
        };
        (0..3)
            .flat_map(|axis| {
                [false, true].map(|tip| Mark {
                    paint: paint(axis),
                    transform: handle(&self.world, axis, mode, tip),
                })
            })
            .collect()
    }

    /// The axis whose handle a line of sight meets first, and how far along.
    pub fn handle_hit(&self, mode: Mode, from: DVec3, toward: DVec3) -> Option<(usize, f64)> {
        (0..3)
            .flat_map(|axis| {
                [false, true].into_iter().filter_map(move |tip| {
                    let inward = handle(&self.world, axis, mode, tip).inverse();
                    let (near, far) = part::through(
                        inward.transform_point3(from),
                        inward.transform_vector3(toward),
                    )?;
                    (near > 0.0 && far >= near).then_some((axis, near))
                })
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Where along a handle's axis a line of sight is: the parameter of
    /// the point of the axis nearest the line, metres from the part's
    /// centre in `world`, the frame a drag started from, so a part moved by
    /// the drag does not move the measure. `None` when the line runs along
    /// the axis.
    fn along(world: &DAffine3, axis: usize, from: DVec3, toward: DVec3) -> Option<f64> {
        let e = [world.x_axis, world.y_axis, world.z_axis][axis].normalize();
        let centre = world.transform_point3(DVec3::new(0.0, 0.0, 0.5));
        let w = centre - from;
        let b = e.dot(toward);
        let denom = 1.0 - b * b;
        if denom < 1e-6 {
            return None;
        }
        Some((b * toward.dot(w) - e.dot(w)) / denom)
    }

    /// The angle of a line of sight about a handle's axis, where the line
    /// crosses the plane through the part's anchor across that axis, in
    /// `world`, the frame a drag started from.
    fn around(world: &DAffine3, axis: usize, from: DVec3, toward: DVec3) -> Option<f64> {
        let axes = [world.x_axis, world.y_axis, world.z_axis].map(|a| a.normalize());
        let (u, v, e) = (axes[(axis + 1) % 3], axes[(axis + 2) % 3], axes[axis]);
        let anchor = world.translation;
        let along = toward.dot(e);
        if along.abs() < 1e-6 {
            return None;
        }
        let t = (anchor - from).dot(e) / along;
        let p = from + toward * t - anchor;
        Some(p.dot(v).atan2(p.dot(u)))
    }

    /// Starts a drag of a handle where the pointer is.
    pub fn press(&mut self, mode: Mode, axis: usize, from: DVec3, toward: DVec3) {
        let start = match mode {
            Mode::Move | Mode::Scale => Self::along(&self.world, axis, from, toward),
            Mode::Rotate => Self::around(&self.world, axis, from, toward),
        };
        if let Some(start) = start {
            self.drag = Some(Drag {
                axis,
                start,
                from: self.world,
            });
        }
    }

    /// Follows a drag with the pointer: the part's world transform as the
    /// drag has it now, or `None` when the pointer says nothing.
    pub fn dragged(
        &self,
        mode: Mode,
        uniform: bool,
        from: DVec3,
        toward: DVec3,
    ) -> Option<DAffine3> {
        let drag = self.drag?;
        let axes = [drag.from.x_axis, drag.from.y_axis, drag.from.z_axis].map(|a| a.normalize());
        let e = axes[drag.axis];
        match mode {
            Mode::Move => {
                let now = Self::along(&drag.from, drag.axis, from, toward)?;
                Some(DAffine3::from_translation(e * (now - drag.start)) * drag.from)
            }
            Mode::Rotate => {
                let now = Self::around(&drag.from, drag.axis, from, toward)?;
                let anchor = drag.from.translation;
                let turn = DQuat::from_axis_angle(e, now - drag.start);
                Some(
                    DAffine3::from_translation(anchor)
                        * DAffine3::from_quat(turn)
                        * DAffine3::from_translation(-anchor)
                        * drag.from,
                )
            }
            Mode::Scale => {
                let now = Self::along(&drag.from, drag.axis, from, toward)?;
                let lengths =
                    [drag.from.x_axis, drag.from.y_axis, drag.from.z_axis].map(|a| a.length());
                // The handle moves with the side it stretches, so the side
                // follows the pointer: a block more of length for a block
                // more of reach.
                let factor = ((lengths[drag.axis] + (now - drag.start)) / lengths[drag.axis])
                    .max(part::MIN_SCALE * BLOCK_M / lengths[drag.axis]);
                let scale = match uniform {
                    true => DVec3::splat(factor),
                    false => {
                        let mut s = DVec3::ONE;
                        s[drag.axis] = factor;
                        s
                    }
                };
                Some(drag.from * DAffine3::from_scale(scale))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::{Part, PartId, Seat, Shape};
    use topology::Sector;

    fn picked(world: DAffine3) -> Gizmo {
        Gizmo {
            id: PartId(1),
            seat: Seat::Sector(Sector::new(0).unwrap()),
            part: Part {
                shape: Shape::Cube,
                pose: client::PartPose::upright([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]),
                paint: 0,
                solid: true,
                shine: 0.0,
            },
            world,
            hot: None,
            drag: None,
        }
    }

    /// A drag is measured against where it started, so a part that moved
    /// under it is where the pointer says and not back and forth.
    #[test]
    fn a_drag_follows_the_pointer_and_does_not_oscillate() {
        let from = DAffine3::from_translation(DVec3::new(10.0, 0.0, 0.0));
        let mut gizmo = picked(from);
        // Looking down from above, the pointer over the x axis.
        let eye = |x: f64| (DVec3::new(x, 0.0, 20.0), -DVec3::Z);
        let (at, toward) = eye(11.0);
        gizmo.press(Mode::Move, 0, at, toward);
        assert!(gizmo.drag.is_some());
        let (at, toward) = eye(13.0);
        let moved = gizmo.dragged(Mode::Move, false, at, toward).unwrap();
        assert!((moved.translation.x - 12.0).abs() < 1e-9, "{moved:?}");
        // The part moved there, the pointer stayed: it stays there.
        gizmo.world = moved;
        let again = gizmo.dragged(Mode::Move, false, at, toward).unwrap();
        assert!((again.translation.x - 12.0).abs() < 1e-9, "{again:?}");
        // Turned a quarter about z from the frame it started in.
        let mut gizmo = picked(from);
        let anchor = from.translation;
        let (at, toward) = (anchor + DVec3::new(5.0, 0.0, 20.0), -DVec3::Z);
        gizmo.press(Mode::Rotate, 2, at, toward);
        let (at, toward) = (anchor + DVec3::new(0.0, 5.0, 20.0), -DVec3::Z);
        let turned = gizmo.dragged(Mode::Rotate, false, at, toward).unwrap();
        let x = turned.transform_vector3(DVec3::X);
        assert!((x - DVec3::Y).length() < 1e-9, "{x:?}");
        gizmo.world = turned;
        let again = gizmo.dragged(Mode::Rotate, false, at, toward).unwrap();
        assert!((again.transform_vector3(DVec3::X) - DVec3::Y).length() < 1e-9);
    }
}
