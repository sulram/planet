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
//! paint and shared by every cube in it. A model of a file (`Shape::Vox`) is
//! asked for over the asset seam when a part first names it, meshed once it
//! arrives, and shared by every part that names it; until it arrives the
//! part stands undrawn, and its box holds a body up.

use std::collections::{BTreeMap, HashMap};

use glam::{DAffine3, DMat3, DVec3};
use part::{Gap, Part, PartId, Pose, Shape};
use protocol::Body;
use scene::{ModelChange, ModelDraw, ModelId, ModelMesh, PictureDraw, VolumeVertex};
use seat::Seat;
use topology::{BLOCK_M, SurfacePoint};
use voxel::{Finish, Paint};
use worldgen::Generator;

use crate::assets::Requests;
use crate::cells::paint_look;
use crate::collision::{BODY_HALF, Footing, STEP_M};
use crate::pictures::{Pictures, Screen};

/// The size of a cube as a hand places it, in blocks: a metre.
pub const CUBE: [f64; 3] = [2.0, 2.0, 2.0];

/// The size of a model's voxel as a hand places it, in blocks: an eighth of
/// a cell, 6.25 cm, so a model of 32 voxels stands 2 m tall
/// (docs/sketches/entities.md § The pose).
pub const VOXEL: f64 = 1.0 / 8.0;

/// The models of the core's own shapes have ids under this; a model of a
/// file is named by the hash of its content, over it.
const SHAPES: u64 = 1 << 16;
/// The bit that marks the model of a file.
const FILE: u64 = 1 << 63;

/// Every part held, by its name, and what draws them.
#[derive(Default)]
pub struct Parts {
    held: BTreeMap<PartId, (Seat, Part)>,
    /// The next name given here, while no world gives them.
    next: u64,
    /// The models announced, by id.
    meshed: Vec<ModelId>,
    changes: Vec<ModelChange>,
    /// The grid of every model of a file that arrived, by the file's hash.
    grids: HashMap<[u8; 32], vox::Grid>,
    /// The files asked for, so none is asked for twice.
    asked: Vec<[u8; 32]>,
    /// The files to ask for, since last asked.
    wanted: Vec<[u8; 32]>,
    /// Where the moon is now, from the planet's centre.
    moon: DVec3,
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
            .filter_map(|(_, part)| self.gap_of(part, column, reach))
            .map(|gap| Footing {
                floor_m: gap.floor.map(|n| n * BLOCK_M),
                ceiling_m: gap.ceiling.map(|n| n * BLOCK_M),
            })
            .reduce(Footing::with)
    }

    /// Every part drawn as a model, placed in the world this frame.
    pub fn drawn(&self, generator: &Generator) -> Vec<ModelDraw> {
        self.all()
            .filter_map(|(_, seat, part)| {
                Some(ModelDraw {
                    mesh: model_of(part)?,
                    transform: self.place_of(generator, seat, &part.pose),
                    glass: Paint::of(part.paint).finish == Finish::Glass,
                })
            })
            .collect()
    }

    /// Every part that is a picture, placed in the world this frame at the
    /// level it is seen at from the screen. A picture in a light finish is
    /// a screen, shining with its own colour; any other is lit as a print.
    pub(crate) fn pictures(
        &self,
        generator: &Generator,
        pictures: &mut Pictures,
        screen: &Screen,
        requests: &mut Requests,
    ) -> Vec<PictureDraw> {
        self.all()
            .filter_map(|(_, seat, part)| {
                let Shape::Image(media) = part.shape else {
                    return None;
                };
                let world_from_quad = self.place_of(generator, seat, &part.pose) * FRONT;
                let shine = f32::from(u8::from(Paint::of(part.paint).finish == Finish::Light));
                Some(pictures.draw(&media, world_from_quad, shine, true, screen, requests))
            })
            .collect()
    }

    pub(crate) fn drain_changes(&mut self) -> Vec<ModelChange> {
        core::mem::take(&mut self.changes)
    }

    /// Announces the model of a part, once. A file's is asked for instead,
    /// and announced when it arrives.
    fn mesh(&mut self, part: &Part) {
        let Some(id) = model_of(part) else {
            return;
        };
        if self.meshed.contains(&id) {
            return;
        }
        match part.shape {
            Shape::Cube => {
                self.meshed.push(id);
                self.changes.push(ModelChange::Add(id, cube(part.paint)));
            }
            Shape::Vox(hash) => self.want(hash),
            // A picture is no model: drawn by `pictures`.
            Shape::Image(_) => {}
        }
    }

    /// Asks for a model's file, once: what a hand wants before it places
    /// the model, to know its size.
    pub(crate) fn want(&mut self, hash: [u8; 32]) {
        if self.grids.contains_key(&hash) || self.asked.contains(&hash) {
            return;
        }
        self.asked.push(hash);
        self.wanted.push(hash);
    }

    /// The files asked for since last taken.
    pub(crate) fn wanted(&mut self) -> Vec<[u8; 32]> {
        core::mem::take(&mut self.wanted)
    }

    /// A model's file arrived: its grid is kept and its mesh announced, the
    /// voxels filling the unit box.
    pub(crate) fn arrived(&mut self, hash: [u8; 32], model: vox::Model) {
        let id = model_of_file(hash);
        if !self.meshed.contains(&id) {
            self.meshed.push(id);
            self.changes
                .push(ModelChange::Add(id, boxed(&model.grid, &model.palette)));
        }
        self.grids.insert(hash, model.grid);
    }

    /// How many voxels a model of a file has along each axis, once it
    /// arrived.
    pub fn size_of(&self, hash: [u8; 32]) -> Option<[u32; 3]> {
        self.grids.get(&hash).map(|grid| grid.size)
    }

    /// What a body standing in a column has under and over it of one part:
    /// its box, and for a model whose file arrived, its own voxels.
    fn gap_of(&self, part: &Part, column: [f64; 2], reach: f64) -> Option<Gap> {
        let boxed = part.gap(column, BODY_HALF, reach)?;
        let Shape::Vox(hash) = part.shape else {
            return Some(boxed);
        };
        match self.grids.get(&hash) {
            Some(grid) => model_gap(grid, part, column, reach),
            None => Some(boxed),
        }
    }
}

/// The model a part is drawn as: the core's shapes in a paint each, a file
/// by its hash. `None` for a shape drawn another way, a picture.
fn model_of(part: &Part) -> Option<ModelId> {
    let shape = match part.shape {
        Shape::Cube => 0,
        Shape::Vox(hash) => return Some(model_of_file(hash)),
        Shape::Image(_) => return None,
    };
    debug_assert!(shape < SHAPES >> 8);
    Some(ModelId((shape << 8) | u64::from(part.paint)))
}

fn model_of_file(hash: [u8; 32]) -> ModelId {
    let head: [u8; 8] = hash[..8].try_into().expect("eight of thirty two");
    ModelId(u64::from_be_bytes(head) | FILE)
}

/// Where a body's feet are read, each way of its column: the middle and the
/// four corners of its width.
const FEET: [[f64; 2]; 5] = [
    [0.0, 0.0],
    [-BODY_HALF, -BODY_HALF],
    [BODY_HALF, -BODY_HALF],
    [-BODY_HALF, BODY_HALF],
    [BODY_HALF, BODY_HALF],
];

/// What a body standing in a column has under its feet and over its head
/// of a model's own voxels, read through its pose: up each foot's line
/// through the part's box, the solid runs of the grid along it. Solid at the
/// height of the step is a wall whose top is the floor offered, below it a
/// floor, above it a roof, as a cube answers (`part::Part::gap`).
fn model_gap(grid: &vox::Grid, part: &Part, column: [f64; 2], reach: f64) -> Option<Gap> {
    let inward = part.pose.inward();
    let up = inward.transform_vector3(DVec3::Z);
    let size = grid.size.map(f64::from);
    let to_grid = |p: DVec3| [(p.x + 0.5) * size[0], (p.y + 0.5) * size[1], p.z * size[2]];
    let mut gap: Option<Gap> = None;
    for [dx, dy] in FEET {
        let foot = inward.transform_point3(DVec3::new(column[0] + dx, column[1] + dy, 0.0));
        let Some((low, high)) = part::through(foot, up) else {
            continue;
        };
        let runs = grid.solid_runs(to_grid(foot + up * low), to_grid(foot + up * high));
        let mut found = Gap {
            floor: None,
            ceiling: None,
        };
        for (a, b) in runs {
            let (h0, h1) = (low + a * (high - low), low + b * (high - low));
            if h0 <= reach && reach <= h1 || h1 <= reach {
                found.floor = Some(found.floor.map_or(h1, |floor| floor.max(h1)));
            } else {
                found.ceiling = Some(found.ceiling.map_or(h0, |roof| roof.min(h0)));
            }
        }
        if found.floor.is_none() && found.ceiling.is_none() {
            continue;
        }
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

fn pick(a: Option<f64>, b: Option<f64>, f: fn(f64, f64) -> f64) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(f(a, b)),
        (a, b) => a.or(b),
    }
}

/// A model's voxels meshed into the unit box: `vox` meshes them in voxels
/// from the middle of the footprint, and here each axis is divided by how
/// many voxels it has. Every colour is the file's own, and no side has an
/// edge drawn.
fn boxed(grid: &vox::Grid, palette: &vox::Palette) -> ModelMesh {
    let mesh = vox::mesh(grid, palette);
    let size = grid.size.map(|n| n as f32);
    let across = [[0, 0], [u8::MAX, 0], [u8::MAX, u8::MAX], [0, u8::MAX]];
    let vertices = mesh
        .vertices
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let [u, w] = across[k % 4];
            VolumeVertex {
                position: [0, 1, 2].map(|i| v.position[i] / size[i]),
                normal: v.normal,
                color: [v.color[0], v.color[1], v.color[2], 0],
                side: [v.sky, u, w, 0],
            }
        })
        .collect();
    ModelMesh {
        vertices,
        indices: mesh.indices,
    }
}
/// The front of a part's box as the picture's quad: the unit quad, `x`
/// across and `y` up, facing `+z`, laid on the box's `y = -0.5` side with
/// its bottom at the anchor, facing `-y`.
const FRONT: DAffine3 = DAffine3 {
    matrix3: DMat3::from_cols(DVec3::X, DVec3::Z, DVec3::NEG_Y),
    translation: DVec3::new(0.0, -0.5, 0.5),
};

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

    #[test]
    fn a_model_is_asked_for_once_and_stood_on_by_its_own_voxels() {
        // A shelter of four voxels across and four up: a pillar at one end
        // and a roof over air wide and high enough for a body.
        let grid = vox::Grid {
            size: [4, 1, 4],
            voxels: vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1],
        };
        let model = vox::Model {
            grid,
            palette: vox::default_palette(),
        };
        let hash = [7; 32];
        let mut parts = Parts::default();
        let seat = Seat::Sector(topology::Sector::new(0).unwrap());
        // A block a voxel: the box is 4 by 1 by 4 blocks, from 998 to 1002
        // across and 10 to 14 up.
        let part = Part {
            shape: Shape::Vox(hash),
            pose: Pose::upright([1000.0, 1000.0, 10.0], [4.0, 1.0, 4.0]),
            paint: 0,
            solid: true,
        };
        parts.place(seat, part);
        parts.place(seat, part);
        assert_eq!(parts.wanted(), vec![hash]);
        assert!(parts.wanted().is_empty());
        assert!(
            parts.drain_changes().is_empty(),
            "nothing is drawn before the file comes"
        );
        // Until it comes, the box holds a body up: on top of it.
        let under_arch = SurfacePoint::new(seat.sector(), 1000.6, 1000.0);
        let boxed = parts.footing(seat, under_arch, 14.0 * BLOCK_M).unwrap();
        assert_eq!(boxed.floor_m, Some(14.0 * BLOCK_M));

        parts.arrived(hash, model);
        assert_eq!(parts.size_of(hash), Some([4, 1, 4]));
        let changes = parts.drain_changes();
        assert_eq!(changes.len(), 1);
        let ModelChange::Add(_, mesh) = &changes[0] else {
            panic!("a mesh is added");
        };
        // Seven voxels, of which some sides touch: fewer than 42 sides, all
        // inside the unit box.
        assert!(mesh.vertices.len() < 42 * 4 && !mesh.vertices.is_empty());
        for v in &mesh.vertices {
            assert!(v.position[0].abs() <= 0.5 && v.position[1].abs() <= 0.5);
            assert!((0.0..=1.0).contains(&v.position[2]));
        }
        // Standing on top: the top of the roof, four blocks up.
        let on_top = parts.footing(seat, under_arch, 14.0 * BLOCK_M).unwrap();
        assert_eq!(on_top.floor_m, Some(14.0 * BLOCK_M));
        // Under the roof, where the box said floor, the voxels say a roof
        // three blocks over the anchor and no floor of the part.
        let feet_m = 10.0 * BLOCK_M;
        let under = parts.footing(seat, under_arch, feet_m).unwrap();
        assert_eq!(under.floor_m, None, "{under:?}");
        assert!(
            (under.ceiling_m.unwrap() - 13.0 * BLOCK_M).abs() < 1e-9,
            "{under:?}"
        );
        // Over the pillar, feet at the anchor: a wall, whose top is the
        // floor offered.
        let pillar = SurfacePoint::new(seat.sector(), 998.5, 1000.0);
        let wall = parts.footing(seat, pillar, feet_m).unwrap();
        assert!(
            (wall.floor_m.unwrap() - 14.0 * BLOCK_M).abs() < 1e-9,
            "{wall:?}"
        );
    }
}
