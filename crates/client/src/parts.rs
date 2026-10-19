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
//!
//! In a world, the world keeps the parts (DECISIONS 111): a part placed here
//! is placed at once and asked of the world, which names it; what the world
//! calls a part is the link's business, and a hand knows a part by the name
//! given here. A set or a remove of a part the world has not named yet
//! waits for the name. What others change arrives as events, and the parts
//! near the body are asked for every second. With no world, a headless
//! picture or a bench, the parts are held here alone.

use std::collections::{BTreeMap, HashMap, HashSet};

use glam::{DAffine3, DVec3};
use part::{Part, PartId, Pose, Shape};
use protocol::parts as wire;
use protocol::{Body, Message};
use scene::{ModelChange, ModelDraw, ModelId, ModelMesh, ModelPass, VolumeVertex};
use seat::Seat;
use topology::{BLOCK_M, SurfacePoint};
use voxel::{Finish, Paint};
use worldgen::Generator;

use crate::cells::{Ask, paint_look};
use crate::collision::{BODY_HALF, Footing, STEP_M};

/// Seconds between two looks at what the world holds near the body.
const LOOK_S: f64 = 1.0;

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

/// A set or a remove asked before the world named the part.
enum Deferred {
    Set(Seat, Part),
    Remove,
}

/// The parts as a world keeps them, from where this client stands.
#[derive(Default)]
struct Link {
    /// This client's own session: a part placed under that number is one
    /// it placed.
    me: u32,
    /// The last number given to an op.
    asked: u32,
    asks: Vec<Ask>,
    /// Places asked that the world has not named, the first first: the
    /// first `placed` of this session's own is the first of them.
    placing: Vec<(u32, PartId)>,
    /// Sets and removes asked, by number: refused, what is held is asked
    /// for again.
    waiting: HashSet<u32>,
    /// What the world calls each part held here, and back.
    world_of: HashMap<PartId, u64>,
    local_of: HashMap<u64, PartId>,
    /// The version the world last said of each part, by its world name.
    versions: HashMap<u64, u64>,
    /// What was asked of a part before the world named it, in order.
    deferred: Vec<(PartId, Deferred)>,
    since_look_s: f64,
}

impl Link {
    /// Queues an op for the world, under the next number.
    fn ask<M: Message>(&mut self, kind: &'static str, op: &M) -> u32 {
        self.asked = self.asked.wrapping_add(1).max(1);
        let id = self.asked;
        self.asks.push(Ask {
            kind,
            payload: op.encode_to_vec(),
            id,
        });
        id
    }
}

/// Every part held, by its name, and what draws them.
#[derive(Default)]
pub struct Parts {
    held: BTreeMap<PartId, (Seat, Part)>,
    /// The next name given here: a hand knows a part by it, in a world or
    /// alone.
    next: u64,
    /// The world that keeps the parts, while one does.
    link: Option<Link>,
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

    /// A world keeps the parts from here on: what was held is forgotten,
    /// and what the world holds near the body is asked for at once.
    pub(crate) fn link(&mut self, me: u32) {
        self.held.clear();
        self.link = Some(Link {
            me,
            since_look_s: LOOK_S,
            ..Link::default()
        });
    }

    /// The world is gone, and what it kept with it.
    pub(crate) fn unlink(&mut self) {
        self.held.clear();
        self.link = None;
    }

    /// What to ask the world since the last call, in order.
    pub(crate) fn drain_asks(&mut self) -> Vec<Ask> {
        self.link
            .as_mut()
            .map(|link| core::mem::take(&mut link.asks))
            .unwrap_or_default()
    }

    /// Asks the world, every second, for the parts near the body this
    /// client lacks or holds stale.
    pub(crate) fn look(&mut self, dt: f64) {
        let Some(link) = &mut self.link else {
            return;
        };
        link.since_look_s += dt;
        if link.since_look_s < LOOK_S {
            return;
        }
        link.since_look_s = 0.0;
        let held = link
            .versions
            .iter()
            .map(|(&id, &version)| wire::Held { id, version })
            .collect();
        link.ask(wire::LOOK, &wire::Look { held });
    }

    /// An event of the world's parts.
    pub(crate) fn receive(&mut self, kind: &str, payload: &[u8]) {
        let Some(me) = self.link.as_ref().map(|link| link.me) else {
            return;
        };
        match kind {
            wire::PLACED => {
                let Ok(placed) = wire::Placed::decode(payload) else {
                    return;
                };
                let Some(record) = &placed.record else {
                    return;
                };
                if placed.session == me {
                    // The first place of this session's own that the world
                    // has not named: this is its name.
                    let link = self.link.as_mut().expect("linked");
                    if !link.placing.is_empty() {
                        let (_, local) = link.placing.remove(0);
                        self.name(local, record);
                        return;
                    }
                }
                self.arrived(record);
            }
            wire::CHANGED => {
                let Ok(changed) = wire::Changed::decode(payload) else {
                    return;
                };
                let Some(record) = &changed.record else {
                    return;
                };
                // A change of this session's own is held already; the
                // version is what is new of it.
                match changed.session == me {
                    true => {
                        let link = self.link.as_mut().expect("linked");
                        link.versions.insert(record.id, record.version);
                    }
                    false => self.arrived(record),
                }
            }
            wire::REMOVED => {
                if let Ok(removed) = wire::Removed::decode(payload) {
                    self.gone(removed.id);
                }
            }
            wire::SEEN => {
                let Ok(seen) = wire::Seen::decode(payload) else {
                    return;
                };
                for record in &seen.records {
                    self.arrived(record);
                }
                for id in seen.gone {
                    self.gone(id);
                }
            }
            _ => {}
        }
    }

    /// The world named a part placed here: what was asked of it meanwhile
    /// goes out under that name.
    fn name(&mut self, local: PartId, record: &wire::Record) {
        let link = self.link.as_mut().expect("linked");
        link.world_of.insert(local, record.id);
        link.local_of.insert(record.id, local);
        link.versions.insert(record.id, record.version);
        let deferred: Vec<Deferred> = {
            let (mine, rest): (Vec<_>, Vec<_>) = core::mem::take(&mut link.deferred)
                .into_iter()
                .partition(|(id, _)| *id == local);
            link.deferred = rest;
            mine.into_iter().map(|(_, deferred)| deferred).collect()
        };
        for deferred in deferred {
            self.send(local, deferred);
        }
    }

    /// Asks the world for a set or a remove of a part it has named, or
    /// keeps it for when it has.
    fn send(&mut self, local: PartId, deferred: Deferred) {
        let Some(link) = &mut self.link else {
            return;
        };
        let Some(&id) = link.world_of.get(&local) else {
            link.deferred.push((local, deferred));
            return;
        };
        let asked = match deferred {
            Deferred::Set(seat, part) => link.ask(
                wire::SET,
                &wire::Set {
                    id,
                    part: Some(seat::part_wire(seat, &part)),
                },
            ),
            Deferred::Remove => link.ask(wire::REMOVE, &wire::Remove { id }),
        };
        link.waiting.insert(asked);
    }

    /// A part as the world holds it: held here under the name it has here,
    /// or a new one.
    fn arrived(&mut self, record: &wire::Record) {
        let Some((seat, part)) = record.part.as_ref().and_then(seat::part_from_wire) else {
            return;
        };
        let link = self.link.as_mut().expect("linked");
        let stale = link.versions.get(&record.id) != Some(&record.version);
        link.versions.insert(record.id, record.version);
        let local = match link.local_of.get(&record.id) {
            Some(&local) => {
                if !stale {
                    return;
                }
                local
            }
            None => {
                self.next += 1;
                let local = PartId(self.next);
                link.local_of.insert(record.id, local);
                link.world_of.insert(local, record.id);
                local
            }
        };
        self.held.insert(local, (seat, part));
        self.mesh(&part);
    }

    /// A part the world no longer holds.
    fn gone(&mut self, id: u64) {
        let Some(link) = &mut self.link else {
            return;
        };
        link.versions.remove(&id);
        if let Some(local) = link.local_of.remove(&id) {
            link.world_of.remove(&local);
            self.held.remove(&local);
        }
    }

    /// The world answered an op of the parts: landed, with no code, or
    /// refused with one. A place refused takes the part back; a set or a
    /// remove refused asks for what is held again. False for a number that
    /// is none of the parts'.
    pub(crate) fn answered(&mut self, id: u32, code: &str) -> bool {
        let Some(link) = &mut self.link else {
            return false;
        };
        if let Some(at) = link.placing.iter().position(|(asked, _)| *asked == id) {
            let (_, local) = link.placing.remove(at);
            if !code.is_empty() {
                self.held.remove(&local);
                link.deferred.retain(|(part, _)| *part != local);
            }
            return true;
        }
        if link.waiting.remove(&id) {
            if !code.is_empty() {
                link.since_look_s = LOOK_S;
            }
            return true;
        }
        false
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

    /// Places a part on a seat and names it. In a world, the world is asked
    /// to place it too, and names it in its own way.
    pub(crate) fn place(&mut self, seat: Seat, part: Part) -> PartId {
        self.next += 1;
        let id = PartId(self.next);
        self.held.insert(id, (seat, part));
        self.mesh(&part);
        if let Some(link) = &mut self.link {
            let place = wire::Place {
                part: Some(seat::part_wire(seat, &part)),
            };
            let asked = link.ask(wire::PLACE, &place);
            link.placing.push((asked, id));
        }
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
        self.send(id, Deferred::Set(seat, part));
        true
    }

    /// Removes a part by its name. False for a name nothing holds.
    pub(crate) fn remove(&mut self, id: PartId) -> bool {
        if self.held.remove(&id).is_none() {
            return false;
        }
        self.send(id, Deferred::Remove);
        true
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
