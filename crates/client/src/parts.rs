//! The parts of a world on the client: the placed things that are no cell,
//! seated on the planet and on the moon, and the models that draw them
//! (DECISIONS 111).
//!
//! A part knows no sphere (`part`). Here it is seated: its pose is said in
//! the flat frame of a point of its seat, and `seat::seated` puts it in the
//! world, rigid, so a part is moved in the world and its pose read back from
//! where it came to (`seat::reseat`). A part stands anywhere on its seat,
//! over a volume, between two, on nature: its place is an address with
//! fractional blocks, never a volume.
//!
//! The parts are a system of the core (DECISIONS 108): anyone reads what
//! they hold, and an op alone changes it. A hand that places, moves and
//! turns them is a plugin's, and it speaks to them through its host
//! (`crate::plugin`): pick, place, set, remove.
//!
//! A part is drawn as a model: a mesh in its own space, a cube of one voxel
//! in one paint, placed by one matrix a frame. A cube's mesh is made once a
//! paint and shared by every cube in it.

use std::collections::BTreeMap;

use glam::{DAffine3, DVec3};
use part::{Part, PartId, Pose, Shape};
use protocol::Body;
use scene::{ModelChange, ModelDraw, ModelId, ModelMesh, ModelPass, VolumeVertex};
use seat::Seat;
use topology::{BLOCK_M, SurfacePoint};
use voxel::{Finish, Paint};
use worldgen::Generator;

use crate::cells::paint_look;
use crate::collision::{BODY_HALF, Footing, STEP_M};

/// The size of a cube as a hand places it, in blocks: a metre.
pub const CUBE: [f64; 3] = [2.0, 2.0, 2.0];

/// Why the parts refuse what was asked of them. A plugin says it at the
/// seam under its own name, and a front end in its own words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartRefusal {
    /// Placing is a builder's and an admin's, and the world named this
    /// session a lower level, or no world has spoken (DECISIONS 104).
    Level,
    /// The pose is no pose: not finite, not a turn, or flat.
    Pose,
    /// No part of that name stands.
    Gone,
}

/// The models of the core's own shapes have ids under this; a model of a
/// file is named by the hash of its content, over it.
const SHAPES: u64 = 1 << 16;

/// Every part held, by its name, and what draws them.
#[derive(Default)]
pub struct Parts {
    held: BTreeMap<PartId, (Seat, Part)>,
    /// The next name given here, while no world gives them.
    next: u64,
    /// The models announced, by id.
    meshed: Vec<ModelId>,
    changes: Vec<ModelChange>,
    /// Where the moon is now, from the planet's centre.
    moon: DVec3,
    /// The marks a hand shows over the world this frame: cubes in a paint,
    /// each where its transform puts it, drawn over everything.
    marks: Vec<ModelDraw>,
}

/// A mark a hand shows over the world: a unit cube in a paint, placed by a
/// world transform from the planet's centre, drawn over everything and
/// cast by nothing. The handles of a gizmo are marks.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mark {
    pub paint: u8,
    pub transform: DAffine3,
}

impl Parts {
    /// The moon is somewhere else: what is seated on it goes with it.
    pub(crate) fn orbit(&mut self, moon: DVec3) {
        self.moon = moon;
    }

    fn centre(&self, body: Body) -> DVec3 {
        match body {
            Body::Moon => self.moon,
            Body::Planet => DVec3::ZERO,
        }
    }

    /// The seat and pose a part standing in the world has: the world
    /// transform from the planet's centre read back onto a body
    /// (`seat::reseat`). `None` for a transform that is no pose.
    pub fn reseat(
        &self,
        generator: &Generator,
        body: Body,
        world: &DAffine3,
    ) -> Option<(Seat, Pose)> {
        let on_body = DAffine3::from_translation(-self.centre(body)) * *world;
        let (sector, pose) = seat::reseat(seat::sphere_of(generator, body), &on_body)?;
        Some((Seat::on(body, sector), pose))
    }

    /// Shows marks over the world, in place of those shown before, and none
    /// with none to show.
    pub(crate) fn mark(&mut self, marks: &[Mark]) {
        self.marks = marks
            .iter()
            .map(|mark| ModelDraw {
                mesh: self.ensure(Shape::Cube, mark.paint),
                transform: mark.transform,
                pass: ModelPass::Over,
            })
            .collect();
    }

    /// A part by its name, and the seat it stands on.
    pub fn get(&self, id: PartId) -> Option<(Seat, &Part)> {
        self.held.get(&id).map(|(seat, part)| (*seat, part))
    }

    /// Every part held, with its name and seat.
    pub fn all(&self) -> impl Iterator<Item = (PartId, Seat, &Part)> {
        self.held
            .iter()
            .map(|(id, (seat, part))| (*id, *seat, part))
    }

    /// Whether any part stands on a body.
    pub fn stands_on(&self, body: Body) -> bool {
        self.held.values().any(|(seat, _)| seat.body() == body)
    }

    /// Places a part on a seat and names it.
    pub(crate) fn place(&mut self, seat: Seat, part: Part) -> PartId {
        self.next += 1;
        let id = PartId(self.next);
        self.held.insert(id, (seat, part));
        self.mesh(&part);
        id
    }

    /// Replaces a part, kept by its name: moved, turned, stretched,
    /// repainted or made solid or not. False for a name nothing holds.
    pub(crate) fn set(&mut self, id: PartId, seat: Seat, part: Part) -> bool {
        let Some(held) = self.held.get_mut(&id) else {
            return false;
        };
        *held = (seat, part);
        self.mesh(&part);
        true
    }

    /// Removes a part by its name. False for a name nothing holds.
    pub(crate) fn remove(&mut self, id: PartId) -> bool {
        self.held.remove(&id).is_some()
    }

    /// The world from a part's own space, with the centre of its body in it.
    pub fn transform(&self, generator: &Generator, id: PartId) -> Option<DAffine3> {
        let (seat, part) = self.get(id)?;
        Some(self.place_of(generator, seat, &part.pose))
    }

    fn place_of(&self, generator: &Generator, seat: Seat, pose: &Pose) -> DAffine3 {
        DAffine3::from_translation(self.centre(seat.body()))
            * seat::seated(seat.sphere(generator), seat.sector(), pose)
    }

    /// The nearest part a line from `from` toward `toward`, unit, meets,
    /// and how far along, metres.
    pub fn pick(&self, generator: &Generator, from: DVec3, toward: DVec3) -> Option<(PartId, f64)> {
        self.all()
            .filter_map(|(id, seat, part)| {
                // The line in the flat frame of the part's anchor, in blocks.
                let frame = seat::frame(seat.sphere(generator), seat.sector(), part.pose.at);
                let inward = frame.inverse();
                let flat_from = inward.transform_point3(from - self.centre(seat.body()))
                    + DVec3::from(part.pose.at);
                let flat_toward = inward.transform_vector3(toward).normalize();
                let blocks = part.hit(flat_from, flat_toward)?;
                Some((id, blocks * BLOCK_M))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// What a body with its feet `feet_m` over the datum, standing in a
    /// column of a seat, has under and over it of the solid parts there.
    /// `None` with none near.
    pub fn footing(&self, seat: Seat, point: SurfacePoint, feet_m: f64) -> Option<Footing> {
        let column = [point.u, point.v];
        let reach = (feet_m + STEP_M) / BLOCK_M;
        self.held
            .values()
            .filter(|(on, part)| *on == seat && part.solid)
            .filter(|(_, part)| {
                // Near enough to be over the body at all: within its longest
                // side of the anchor, and the body's width.
                let span = part.pose.scale.iter().fold(0.0, |a: f64, &b| a.max(b)) + BODY_HALF;
                let [u, v, _] = part.pose.at;
                (u - column[0]).abs() <= span && (v - column[1]).abs() <= span
            })
            .filter_map(|(_, part)| part.gap(column, BODY_HALF, reach))
            .map(|gap| Footing {
                floor_m: gap.floor.map(|n| n * BLOCK_M),
                ceiling_m: gap.ceiling.map(|n| n * BLOCK_M),
            })
            .reduce(Footing::with)
    }

    /// Every part, placed in the world this frame, and the marks over them.
    pub fn drawn(&self, generator: &Generator) -> Vec<ModelDraw> {
        self.all()
            .map(|(_, seat, part)| ModelDraw {
                mesh: model_of(part),
                transform: self.place_of(generator, seat, &part.pose),
                pass: match Paint::of(part.paint).finish {
                    Finish::Glass => ModelPass::Glass,
                    Finish::Matte | Finish::Light => ModelPass::Solid,
                },
            })
            .chain(self.marks.iter().copied())
            .collect()
    }

    pub(crate) fn drain_changes(&mut self) -> Vec<ModelChange> {
        core::mem::take(&mut self.changes)
    }

    /// Announces the model of a part, once.
    fn mesh(&mut self, part: &Part) {
        self.ensure(part.shape, part.paint);
    }

    /// The model of a shape in a paint, announced the first time it is
    /// asked for.
    fn ensure(&mut self, shape: Shape, paint: u8) -> ModelId {
        let id = model_id(shape, paint);
        if !self.meshed.contains(&id) {
            self.meshed.push(id);
            let mesh = match shape {
                Shape::Cube => cube(paint),
            };
            self.changes.push(ModelChange::Add(id, mesh));
        }
        id
    }
}

/// The model a part is drawn as.
fn model_of(part: &Part) -> ModelId {
    model_id(part.shape, part.paint)
}

/// The model of one of the core's shapes in a paint.
fn model_id(shape: Shape, paint: u8) -> ModelId {
    let shape = match shape {
        Shape::Cube => 0,
    };
    debug_assert!(shape < SHAPES >> 8);
    ModelId((shape << 8) | u64::from(paint))
}

/// A cube of one voxel in a paint: the unit box, its sides wound counter
/// clockwise seen from outside, every corner open to the sky.
fn cube(paint: u8) -> ModelMesh {
    let (color, edge) = paint_look(paint);
    // Each side from its first corner: across, then up, then back.
    let sides: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [0.0, 0.0, 1.0],
            [
                [-0.5, -0.5, 1.0],
                [0.5, -0.5, 1.0],
                [0.5, 0.5, 1.0],
                [-0.5, 0.5, 1.0],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [-0.5, -0.5, 0.0],
                [-0.5, 0.5, 0.0],
                [0.5, 0.5, 0.0],
                [0.5, -0.5, 0.0],
            ],
        ),
        (
            [1.0, 0.0, 0.0],
            [
                [0.5, -0.5, 0.0],
                [0.5, 0.5, 0.0],
                [0.5, 0.5, 1.0],
                [0.5, -0.5, 1.0],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [-0.5, -0.5, 0.0],
                [-0.5, -0.5, 1.0],
                [-0.5, 0.5, 1.0],
                [-0.5, 0.5, 0.0],
            ],
        ),
        (
            [0.0, 1.0, 0.0],
            [
                [-0.5, 0.5, 0.0],
                [-0.5, 0.5, 1.0],
                [0.5, 0.5, 1.0],
                [0.5, 0.5, 0.0],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [-0.5, -0.5, 0.0],
                [0.5, -0.5, 0.0],
                [0.5, -0.5, 1.0],
                [-0.5, -0.5, 1.0],
            ],
        ),
    ];
    let across = [[0, 0], [u8::MAX, 0], [u8::MAX, u8::MAX], [0, u8::MAX]];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (normal, corners) in sides {
        let base = vertices.len() as u32;
        for (position, [u, v]) in corners.into_iter().zip(across) {
            vertices.push(VolumeVertex {
                position,
                normal,
                color,
                side: [u8::MAX, u, v, edge],
            });
        }
        indices.extend([0, 1, 2, 0, 2, 3].map(|k| base + k));
    }
    ModelMesh { vertices, indices }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn a_cube_is_six_sides_wound_outward() {
        let mesh = cube(5);
        assert_eq!((mesh.vertices.len(), mesh.indices.len()), (24, 36));
        for side in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(mesh.vertices[side[i] as usize].position));
            let normal = Vec3::from(mesh.vertices[side[0] as usize].normal);
            let wound = (b - a).cross(c - a);
            assert!(wound.dot(normal) > 0.0, "{side:?} winds against its normal");
            // Every corner is on the side its normal names.
            let centre = Vec3::new(0.0, 0.0, 0.5);
            assert!(((a - centre).dot(normal) - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn a_body_stands_on_a_solid_cube_and_not_on_an_open_one() {
        let generator = Generator::new(worldgen::Recipe::new(1)).unwrap();
        let mut parts = Parts::default();
        let seat = Seat::Sector(topology::Sector::new(0).unwrap());
        let at = [1000.0, 1000.0, 10.0];
        let part = Part {
            shape: Shape::Cube,
            pose: Pose::upright(at, CUBE),
            paint: 0,
            solid: true,
        };
        let id = parts.place(seat, part);
        let point = SurfacePoint::new(seat.sector(), 1000.3, 999.8);
        let top_m = 12.0 * BLOCK_M;
        let footing = parts.footing(seat, point, top_m).unwrap();
        assert_eq!(footing.floor_m, Some(top_m));
        // Under it, a roof.
        let under = parts.footing(seat, point, 0.0).unwrap();
        assert_eq!(under.ceiling_m, Some(10.0 * BLOCK_M));
        // A step away, nothing.
        assert!(
            parts
                .footing(
                    seat,
                    SurfacePoint::new(seat.sector(), 1003.0, 1000.0),
                    top_m
                )
                .is_none()
        );
        // Made open, nothing to stand on.
        parts.set(
            id,
            seat,
            Part {
                solid: false,
                ..part
            },
        );
        assert!(parts.footing(seat, point, top_m).is_none());
        // Aimed at from above in the world, it is met at its top.
        let world = parts.transform(&generator, id).unwrap();
        let top = world.transform_point3(DVec3::new(0.0, 0.0, 1.0));
        let up = top.normalize();
        let (hit, away_m) = parts.pick(&generator, top + up * 3.0, -up).unwrap();
        assert_eq!(hit, id);
        assert!((away_m - 3.0).abs() < 1e-6, "{away_m}");
        // One model for every cube in a paint.
        parts.place(seat, part);
        assert_eq!(parts.drain_changes().len(), 1);
    }
}
