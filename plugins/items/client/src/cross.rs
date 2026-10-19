//! The crossing: a body that walks through a portal's pane is taken to
//! stand on the landmark it leads to, facing the way the star points,
//! through the one `go_to` a link and the chart travel by. Read in the
//! plugin's turn, after the body moved: the line from where the feet stood
//! last turn to where they stand now passes the pane's plane inside its
//! doorway. A leap, a link's or a portal's, is no walk: it starts the line
//! over, and the crossing is armed again only once the body has stood clear
//! of every pane, so a door on the star one lands on does not send the
//! body straight back.

use client::{Host, PartId, Shape};
use glam::{DAffine3, DVec3};

use crate::{CROSSED, Crossed, Items, Landmarked};

/// How far a body walks in one turn at most, metres: further is a leap,
/// never a walk through a door.
const STRIDE_M: f64 = 8.0;
/// How far from the body a portal is read, metres: a plot's width.
const NEAR_M: f64 = 32.0;
/// How far from a pane the body stands clear of it, metres.
const CLEAR_M: f64 = 1.0;

impl Items {
    /// Reads the body's last step against every portal near it, and leaps
    /// through the first pane it crossed.
    pub(crate) fn cross(&mut self, host: &mut Host<'_>) {
        let now = host.body().position;
        let before = self.feet.replace(now);
        let doors: Vec<(PartId, Option<PartId>, DAffine3)> = host
            .parts()
            .all()
            .filter_map(|(id, _, part)| {
                let Shape::Portal(to) = part.shape else {
                    return None;
                };
                let world = host.part_transform(id)?;
                ((world.translation - now).length() <= NEAR_M).then_some((id, to, world))
            })
            .collect();
        if !self.armed {
            self.armed = doors.iter().all(|(_, _, world)| !within(world, now));
            return;
        }
        let Some(before) = before else {
            return;
        };
        if (now - before).length() > STRIDE_M {
            return;
        }
        let crossed = doors
            .iter()
            .find(|(_, _, world)| through(world, before, now))
            .map(|&(id, to, _)| (id, to));
        if let Some((portal, to)) = crossed {
            self.leap(portal, to, host);
        }
    }

    /// Takes the body where a portal leads: to the landmark held here, at
    /// once; to one this client does not hold, once the chart says where it
    /// stands; nowhere, and the body stays and the seam hears it.
    fn leap(&mut self, portal: PartId, to: Option<PartId>, host: &mut Host<'_>) {
        let Some(to) = to else {
            return self.say_crossed(portal, None, None, host);
        };
        let held = host
            .parts()
            .get(to)
            .filter(|(_, part)| matches!(part.shape, Shape::Landmark(_)))
            .map(|(seat, part)| (seat, *part));
        match held {
            Some((seat, part)) => {
                let place = host.place_at(seat, &part.pose);
                self.arrive(portal, to, place, host);
            }
            None => {
                self.pending = Some((portal, to));
                host.chart();
            }
        }
    }

    /// The chart answered: a crossing that waited on it leaps to the
    /// landmark charted, or nowhere when the world no longer has it.
    pub(crate) fn charted(&mut self, charted: &[Landmarked], host: &mut Host<'_>) {
        let Some((portal, to)) = self.pending.take() else {
            return;
        };
        match charted.iter().find(|landmark| landmark.id == to.0) {
            Some(landmark) => {
                let place = host.place_at(landmark.seat, &landmark.part.pose);
                self.arrive(portal, to, place, host);
            }
            None => self.say_crossed(portal, Some(to), None, host),
        }
    }

    fn arrive(&mut self, portal: PartId, to: PartId, place: String, host: &mut Host<'_>) {
        match host.go_to(&place) {
            Ok(()) => {
                self.feet = None;
                self.armed = false;
                self.say_crossed(portal, Some(to), Some(place), host);
            }
            Err(message) => host.reject(message),
        }
    }

    fn say_crossed(
        &self,
        portal: PartId,
        to: Option<PartId>,
        place: Option<String>,
        host: &mut Host<'_>,
    ) {
        host.emit(
            CROSSED,
            &Crossed {
                portal: portal.0,
                to: to.map(|to| to.0),
                place,
            },
        );
    }
}

/// Whether a step from `before` to `now` passes a portal's pane, its box's
/// `y = 0` plane, inside the doorway: between the box's sides along `x` and
/// `z` where the step meets the plane.
fn through(world: &DAffine3, before: DVec3, now: DVec3) -> bool {
    let normal = world.matrix3.y_axis.normalize_or_zero();
    let d0 = (before - world.translation).dot(normal);
    let d1 = (now - world.translation).dot(normal);
    if d0 == d1 || d0.signum() == d1.signum() {
        return false;
    }
    let met = before + (now - before) * (d0 / (d0 - d1));
    in_doorway(world, met)
}

/// Whether a point stands within a metre of a portal's pane, inside its
/// doorway.
fn within(world: &DAffine3, point: DVec3) -> bool {
    let normal = world.matrix3.y_axis.normalize_or_zero();
    let away_m = (point - world.translation).dot(normal);
    away_m.abs() <= CLEAR_M && in_doorway(world, point)
}

/// How far under a door's sill the feet still pass through it, metres: a
/// step, since the feet stand on the ground and the sill is placed on it,
/// and on a slope the one is under the other by a little.
const SILL_M: f64 = 0.5;

/// Whether a point, dropped onto the pane's plane, is inside the doorway:
/// between the box's sides along `x`, between a step under its sill and
/// its top along `z`.
fn in_doorway(world: &DAffine3, point: DVec3) -> bool {
    let local = world.inverse().transform_point3(point);
    let sill = SILL_M / world.matrix3.z_axis.length();
    local.x.abs() <= 0.5 && (-sill..=1.0).contains(&local.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door() -> DAffine3 {
        // A doorway a metre wide and two tall, standing at the origin, its
        // pane on the `xz` plane.
        DAffine3::from_scale(DVec3::new(1.0, 0.05, 2.0))
    }

    #[test]
    fn a_step_through_the_doorway_crosses_and_one_beside_it_does_not() {
        let world = door();
        assert!(through(
            &world,
            DVec3::new(0.2, -1.0, 0.1),
            DVec3::new(0.2, 1.0, 0.1)
        ));
        assert!(
            through(
                &world,
                DVec3::new(0.2, 1.0, 0.1),
                DVec3::new(0.2, -1.0, 0.1)
            ),
            "from either side"
        );
        assert!(
            !through(
                &world,
                DVec3::new(0.8, -1.0, 0.1),
                DVec3::new(0.8, 1.0, 0.1)
            ),
            "past the post"
        );
        assert!(
            !through(
                &world,
                DVec3::new(0.2, -1.0, 2.5),
                DVec3::new(0.2, 1.0, 2.5)
            ),
            "over the lintel"
        );
        assert!(
            through(
                &world,
                DVec3::new(0.2, -1.0, -0.4),
                DVec3::new(0.2, 1.0, -0.4)
            ),
            "feet a little under the sill"
        );
        assert!(
            !through(
                &world,
                DVec3::new(0.2, -1.0, -0.6),
                DVec3::new(0.2, 1.0, -0.6)
            ),
            "under the floor"
        );
        assert!(
            !through(
                &world,
                DVec3::new(0.2, -2.0, 0.1),
                DVec3::new(0.2, -1.0, 0.1)
            ),
            "short of it"
        );
        assert!(
            !through(
                &world,
                DVec3::new(0.2, -1.0, 0.1),
                DVec3::new(0.2, -1.0, 0.1)
            ),
            "standing still"
        );
        // A step that meets the plane inside the doorway crosses, wherever it
        // starts and ends.
        assert!(through(
            &world,
            DVec3::new(-0.9, -1.0, 0.1),
            DVec3::new(0.9, 1.0, 0.1)
        ));
    }

    #[test]
    fn a_body_stands_clear_a_metre_from_the_pane() {
        let world = door();
        assert!(within(&world, DVec3::new(0.0, 0.0, 0.0)));
        assert!(within(&world, DVec3::new(0.3, 0.9, 1.5)));
        assert!(!within(&world, DVec3::new(0.3, 1.1, 1.5)));
        assert!(
            !within(&world, DVec3::new(0.7, 0.0, 1.5)),
            "beside the door"
        );
    }
}
