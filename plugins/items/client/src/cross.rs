//! The crossing: a body that walks through a portal's pane is taken to
//! stand on the landmark it leads to, facing the way the star points,
//! through the one `go_to` a link and the chart travel by. Read in the
//! plugin's turn, after the body moved: the line from where the feet stood
//! last turn to where they stand now passes the pane's plane where the
//! body, feet to head, meets the ring's circle. A leap, a link's or a
//! portal's, is no walk: it starts the line over, and the crossing is armed
//! again only once the body has stood clear of every pane, so a portal on
//! the star one lands on does not send the body straight back.

use client::{BODY_M, Host, PartId, Shape};
use glam::{DAffine3, DVec3};

use crate::{CROSSED, Crossed, Items, Landmarked};

/// How far a body walks in one turn at most, metres: further is a leap,
/// never a walk through a ring.
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
        let rings: Vec<(PartId, Option<PartId>, DAffine3)> = host
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
            self.armed = rings.iter().all(|(_, _, world)| !within(world, now));
            return;
        }
        let Some(before) = before else {
            return;
        };
        if (now - before).length() > STRIDE_M {
            return;
        }
        let crossed = rings
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
/// `y = 0` plane, through the ring: where the step meets the plane, the
/// body standing there, feet to head, meets the circle.
fn through(world: &DAffine3, before: DVec3, now: DVec3) -> bool {
    let normal = world.matrix3.y_axis.normalize_or_zero();
    let d0 = (before - world.translation).dot(normal);
    let d1 = (now - world.translation).dot(normal);
    if d0 == d1 || d0.signum() == d1.signum() {
        return false;
    }
    let met = before + (now - before) * (d0 / (d0 - d1));
    in_ring(world, met)
}

/// Whether a body standing at a point is within a metre of a portal's pane
/// and meets its circle.
fn within(world: &DAffine3, point: DVec3) -> bool {
    let normal = world.matrix3.y_axis.normalize_or_zero();
    let away_m = (point - world.translation).dot(normal);
    away_m.abs() <= CLEAR_M && in_ring(world, point)
}

/// Whether a body whose feet stand at a point, dropped onto the pane's
/// plane, meets the ring's circle: the circle fills the box along `x` and
/// `z`, and the body is a column from the feet up to [`BODY_M`], so a
/// body passes a ring its feet are under the rim of, as a body does.
fn in_ring(world: &DAffine3, feet: DVec3) -> bool {
    let local = world.inverse().transform_point3(feet);
    let x = local.x;
    if x.abs() > 0.5 {
        return false;
    }
    let half = (0.25 - x * x).sqrt();
    let head = local.z + BODY_M / world.matrix3.z_axis.length();
    local.z <= 0.5 + half && head >= 0.5 - half
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring() -> DAffine3 {
        // A ring two metres across, standing at the origin, its pane on
        // the `xz` plane.
        DAffine3::from_scale(DVec3::new(2.0, 0.05, 2.0))
    }

    #[test]
    fn a_step_through_the_ring_crosses_and_one_beside_it_does_not() {
        let world = ring();
        let step = |x: f64, z: f64| through(&world, DVec3::new(x, -1.0, z), DVec3::new(x, 1.0, z));
        assert!(step(0.2, 0.1));
        assert!(
            through(
                &world,
                DVec3::new(0.2, 1.0, 0.1),
                DVec3::new(0.2, -1.0, 0.1)
            ),
            "from either side"
        );
        assert!(
            step(0.8, 0.0),
            "off the middle, the body still meets the circle"
        );
        assert!(!step(1.1, 0.0), "past the rim");
        assert!(!step(0.2, 2.5), "over the ring");
        assert!(
            step(0.2, -0.4),
            "feet a little under the ring: the body meets it"
        );
        assert!(!step(0.2, -2.0), "under the ground");
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
        // A step that meets the plane inside the ring crosses, wherever it
        // starts and ends.
        assert!(through(
            &world,
            DVec3::new(-0.9, -1.0, 0.1),
            DVec3::new(0.9, 1.0, 0.1)
        ));
    }

    #[test]
    fn a_body_stands_clear_a_metre_from_the_pane() {
        let world = ring();
        assert!(within(&world, DVec3::new(0.0, 0.0, 0.0)));
        assert!(within(&world, DVec3::new(0.3, 0.9, 1.5)));
        assert!(!within(&world, DVec3::new(0.3, 1.1, 1.5)));
        assert!(
            !within(&world, DVec3::new(1.2, 0.0, 0.5)),
            "beside the ring"
        );
    }
}
