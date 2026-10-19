//! Placing an item: a click puts it where the pointer meets the world, on a
//! side of a cell, on a part or on the ground, standing up (DECISIONS 111).
//! A model stands as big as its file says, a voxel an eighth of a cell. A
//! picture stands a metre tall in its own aspect, its face turned to the
//! eye, as a pane nobody collides with. Placed, the part is picked, so the
//! hand that put it there moves it next.

use client::{CUBE, Eye, Feet, Host, PartPose, REACH_M, Shape, VOXEL};
use glam::{DQuat, DVec3};
use topology::BLOCK_M;

use crate::{Item, Items, Refusal, refuse};

/// How thick a picture's pane is, in blocks: enough to pick, nothing to see.
const PANE: f64 = 0.1;

impl Items {
    /// Places the item in hand where the eye's pointer meets the world.
    pub(crate) fn place(&mut self, item: Item, eye: &Eye, host: &mut Host<'_>) {
        let (from, toward) = eye.sight();
        let Some(feet) = met(host, from, toward) else {
            return;
        };
        let at = [feet.point.u, feet.point.v, feet.height_m / BLOCK_M];
        let part = match item {
            Item::Cube => client::Part {
                shape: Shape::Cube,
                pose: PartPose::upright(at, CUBE),
                paint: self.made_of(),
                solid: true,
            },
            Item::Model => {
                // With no file taken there is nothing to place yet.
                let Some(hash) = self.model else {
                    return;
                };
                let Some(size) = host.model_size(hash) else {
                    return refuse(host, Refusal::Unread);
                };
                client::Part {
                    shape: Shape::Vox(hash),
                    pose: PartPose::upright(at, size.map(|n| f64::from(n) * VOXEL)),
                    paint: self.made_of(),
                    solid: true,
                }
            }
            Item::Picture => {
                // Nothing chosen yet: the panel asks for a file.
                let Some(media) = self.picture else {
                    return;
                };
                // The picture's front, its box's `-y` side, turned to face
                // where the eye stands, by the eye's column of the seat.
                let eye = host.feet_at(from);
                let toward = [eye.point.u - at[0], eye.point.v - at[1]];
                let yaw = libm::atan2(toward[0], -toward[1]);
                client::Part {
                    shape: Shape::Image(media),
                    pose: PartPose {
                        at,
                        rotation: DQuat::from_rotation_z(yaw),
                        scale: [CUBE[2] * media.aspect(), PANE, CUBE[2]],
                    },
                    paint: self.made_of(),
                    solid: false,
                }
            }
        };
        match host.place_part(feet.seat, part) {
            Ok(id) => {
                self.take(None, host);
                self.pick(Some(id), host);
            }
            Err(refusal) => refuse(host, refusal.into()),
        }
    }
}

/// Where a line of sight meets the world, as feet standing there: the
/// nearest of a side of a cell, a part and the ground. Met on a side of a
/// cell that is no top, the feet stand off it by half a cube, so the cube
/// placed there touches the side.
fn met(host: &Host<'_>, from: DVec3, toward: DVec3) -> Option<Feet> {
    let mut nearest: Option<(f64, Feet)> = None;
    let mut offer = |away_m: f64, feet: Feet| {
        if nearest.is_none_or(|(near_m, _)| away_m < near_m) {
            nearest = Some((away_m, feet));
        }
    };
    if let Some(aim) = host.sight(from, toward, None).aim() {
        let (cell, face) = (aim.hit.cell, aim.hit.face);
        let axis = face.axis;
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        let corner = |da: i32, db: i32| {
            let mut p = cell;
            p[axis] += i32::from(face.positive);
            p[a] += da;
            p[b] += db;
            host.corner(aim.seat, p)
        };
        let (c0, c1, c3) = (corner(0, 0), corner(1, 0), corner(0, 1));
        let centre = c0 + (c1 - c0) / 2.0 + (c3 - c0) / 2.0;
        let normal = (c1 - c0).cross(c3 - c0).normalize_or_zero();
        let outward = match face.positive {
            true => normal,
            false => -normal,
        };
        let along = toward.dot(outward);
        if along < -1e-9 {
            let away_m = (centre - from).dot(outward) / along;
            let top = axis == 2 && face.positive;
            let off = if top {
                DVec3::ZERO
            } else {
                outward * (CUBE[0] / 2.0 * BLOCK_M)
            };
            offer(away_m, host.feet_at(from + toward * away_m + off));
        }
    }
    if let Some((_, away_m)) = host.pick(from, toward) {
        offer(away_m, host.feet_at(from + toward * away_m));
    }
    if let Some((feet, away_m)) = host.ground_hit(from, toward, REACH_M) {
        offer(away_m, feet);
    }
    nearest.map(|(_, feet)| feet)
}
