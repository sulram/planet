//! Placing an item: a click puts it where the pointer meets the world, on a
//! side of a cell, on a part or on the ground, standing up (DECISIONS 111).
//! A model stands as big as its file says, a voxel an eighth of a cell. A
//! picture stands a metre tall in its own aspect, its face turned to the
//! eye, as a pane nobody collides with; a portal stands as a ring two
//! metres across, turned the same way. Placed, the part is picked, so
//! the hand that put it there moves it next.

use client::{CUBE, Eye, Feet, Host, PartPose, REACH_M, Shape, VOXEL};
use glam::{DQuat, DVec3};
use topology::BLOCK_M;

/// How far in front of the body a file given is placed, metres, and how
/// high over the feet the line down to the ground starts, so a thing
/// stands on what is there, a floor over the ground too.
const AHEAD_M: f64 = 2.5;
const OVER_M: f64 = 6.0;
/// How far under the feet the ground may be for a thing placed in front
/// of the body to stand on it, metres: a flyer a little over a floor, or
/// over the top of a platform, still builds on it. Further down, the body
/// is in the air on purpose, and the thing floats at its own height in
/// front of it rather than landing on a ground it cannot see.
const HOLD_M: f64 = 2.0;

use crate::{Item, Items, Refusal, refuse};

/// How thick a picture's pane is, in blocks: enough to pick, nothing to see.
const PANE: f64 = 0.1;
/// A landmark's star, in blocks: a metre across, eight centimetres thick.
const STAR: [f64; 3] = [2.0, 2.0, 0.16];
/// A portal's ring, in blocks: two metres across, a pane thick.
const RING: [f64; 3] = [2.0 * CUBE[0], PANE, 2.0 * CUBE[2]];

impl Items {
    /// Places the item in hand where the eye's pointer meets the world.
    pub(crate) fn place(&mut self, item: Item, eye: &Eye, host: &mut Host<'_>) {
        let (from, toward) = eye.sight();
        self.place_along(item, from, toward, host);
    }

    /// Places the item in hand in front of the body, with no click: a line
    /// from over the body's front straight down meets the ground, a cell
    /// or a part, and the item stands there when that is within [`HOLD_M`]
    /// under the feet; further down, the item floats in front of the body
    /// at its own height. A file given lands so.
    pub(crate) fn place_ahead(&mut self, item: Item, host: &mut Host<'_>) {
        let body = host.body();
        let flat = (body.facing - body.up * body.facing.dot(body.up)).normalize_or(body.facing);
        let ahead = body.position + flat * AHEAD_M;
        let from = ahead + body.up * OVER_M;
        let own = host.feet();
        let feet = met(host, from, -body.up)
            .filter(|met| {
                met.seat.body() == own.seat.body() && own.height_m - met.height_m <= HOLD_M
            })
            .unwrap_or_else(|| host.feet_at(ahead));
        self.place_at(item, feet, from, host);
    }

    /// Places the item in hand where a line meets the world.
    fn place_along(&mut self, item: Item, from: DVec3, toward: DVec3, host: &mut Host<'_>) {
        let Some(feet) = met(host, from, toward) else {
            return;
        };
        self.place_at(item, feet, from, host);
    }

    /// Places the item in hand standing at some feet, its front turned to
    /// where the line that found them started, the eye.
    fn place_at(&mut self, item: Item, feet: Feet, from: DVec3, host: &mut Host<'_>) {
        let at = [feet.point.u, feet.point.v, feet.height_m / BLOCK_M];
        let part = match item {
            Item::Cube => client::Part {
                shape: Shape::Cube,
                pose: PartPose::upright(at, CUBE),
                paint: self.made_of(),
                solid: true,
                shine: 0.0,
                reach_m: 0.0,
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
                    shine: 0.0,
                    reach_m: 0.0,
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
                    shine: 0.0,
                    reach_m: 0.0,
                }
            }
            Item::Portal => {
                // A ring two metres across, its front, its box's `-y` side,
                // turned to face where the eye stands, as a picture is.
                let eye = host.feet_at(from);
                let toward = [eye.point.u - at[0], eye.point.v - at[1]];
                let yaw = libm::atan2(toward[0], -toward[1]);
                let shape = Shape::Portal(client::Portal {
                    to: self.to,
                    rim: self.rim,
                });
                client::Part {
                    shape,
                    pose: PartPose {
                        at,
                        rotation: DQuat::from_rotation_z(yaw),
                        scale: RING,
                    },
                    paint: self.paint_of(shape),
                    solid: false,
                    shine: 0.0,
                    reach_m: 0.0,
                }
            }
            Item::Landmark => {
                let shape = Shape::Landmark(self.name_in_hand());
                client::Part {
                    shape,
                    pose: PartPose::upright(at, STAR),
                    paint: self.paint_of(shape),
                    solid: false,
                    shine: 0.0,
                    reach_m: 0.0,
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
/// cell that is no top, the feet stand off it so the cube placed there
/// touches the side: by half a cube from a wall, by a whole one under a
/// ceiling, since a cube rises from its feet.
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
            let off = match (axis == 2, face.positive) {
                (true, true) => DVec3::ZERO,
                (true, false) => outward * (CUBE[2] * BLOCK_M),
                (false, _) => outward * (CUBE[0] / 2.0 * BLOCK_M),
            };
            offer(away_m, host.feet_at(from + toward * away_m + off));
        }
    }
    // On the top of a part the item stands; under it, it hangs; on a side,
    // it stands off by half its width, as on a cell's side.
    if let Some(hit) = host.hit_part(from, toward) {
        let off = match (hit.side.z > 0.5, hit.side.z < -0.5) {
            (true, _) => DVec3::ZERO,
            (_, true) => hit.outward * (CUBE[2] * BLOCK_M),
            _ => hit.outward * (CUBE[0] / 2.0 * BLOCK_M),
        };
        offer(hit.away_m, host.feet_at(from + toward * hit.away_m + off));
    }
    if let Some((feet, away_m)) = host.ground_hit(from, toward, REACH_M) {
        offer(away_m, feet);
    }
    nearest.map(|(_, feet)| feet)
}
