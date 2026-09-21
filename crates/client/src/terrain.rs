//! Terrain streaming: a quadtree per sector, ground to orbit, for any body.
//!
//! Each node is a square of address space meshed as one patch of
//! `PATCH_GRID^2` quads. Near the camera nodes split, so triangles stay about
//! the same size on screen from the ground to orbit. Patches are generated a
//! few per frame, closest first; until its children are ready a node keeps
//! drawing itself, so there are never holes.
//!
//! This is the far field of the terrain layer: a heightfield of the
//! generator's ground, sea floor included, sampled with the generator's LOD
//! filter so a coarse patch is a smooth version of the fine one. Where a patch
//! dips under a sea it also carries a water surface. Editable terrain near the
//! player (density + surface nets, ROADMAP M1/M3) will replace the deepest
//! levels, not this structure.
//!
//! Everything here is relative to the centre of the body: the planet sits at
//! the world origin, the moon moves, and whoever draws a patch adds the body's
//! centre for that frame.

use std::collections::HashMap;

use glam::{DVec3, Vec3};
use scene::{
    Camera, PATCH_GRID, PATCH_VERTICES, PatchId, TerrainChange, TerrainMesh, TerrainVertex,
    WaterVertex,
};
use topology::{BLOCK_M, RADIUS_M, SECTOR_BITS, SECTOR_SIDE, Sector, SurfacePoint};
use voxel::CHUNK_SIDE;
use worldgen::{Generator, MOON_RADIUS_M, Material, Sample};

/// A node splits when the camera is closer than this many node widths.
const SPLIT_DISTANCE: f64 = 2.4;
/// Work generated per update, in units of a heightfield patch, each of which
/// costs about a millisecond.
const BUILD_BUDGET: usize = 6;
/// What a volume patch costs in those units. It carries fifty times the
/// samples of the height it replaces, so counting patches instead of work let
/// six of them into one frame: the descent blew from 9 ms to 42 against a
/// budget of 12, which is what the bench is for.
const VOLUME_COST: usize = 6;
/// Patches around a body under the ground that are meshed down to it, as a
/// Chebyshev radius in patches of the deepest level, each 16 m across. The
/// frame does not feel it (the budget still spends one volume patch a frame),
/// so what it costs is how long the ground takes to arrive and how many
/// patches are held.
const UNDER_RING: i64 = 4;
/// Patches kept before the least recently used ones are dropped.
const CACHE_PATCHES: usize = 1400;

const G: i32 = PATCH_GRID as i32;

/// The body a terrain streamer covers. Every body is a quad sphere meshed by
/// the same quadtree; they differ in size, in depth, and in having a sea.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Body {
    Planet,
    Moon,
}

impl Body {
    pub fn radius_m(self) -> f64 {
        match self {
            Body::Planet => RADIUS_M,
            Body::Moon => MOON_RADIUS_M,
        }
    }

    /// Deepest quadtree level. The planet: patches of 32 blocks, one vertex
    /// per block. The moon is smaller, so it gets as fine sooner.
    fn max_depth(self) -> u32 {
        match self {
            Body::Planet => SECTOR_BITS - 5,
            Body::Moon => 9,
        }
    }

    fn has_sea(self) -> bool {
        self == Body::Planet
    }

    /// No ground lies deeper than this under the datum, whatever the recipe.
    fn deepest_m(self) -> f64 {
        match self {
            Body::Planet => 2000.0,
            Body::Moon => 1500.0,
        }
    }

    pub fn sample(self, generator: &Generator, direction: [f64; 3], footprint_m: f64) -> Sample {
        match self {
            Body::Planet => generator.sample_at(direction, footprint_m),
            Body::Moon => generator.moon_sample_at(direction, footprint_m),
        }
    }
}

/// A quadtree node: a square of `SECTOR_SIDE >> depth` cells of one body.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Node {
    body: Body,
    sector: Sector,
    depth: u32,
    x: u32,
    y: u32,
}

impl Node {
    fn id(self) -> PatchId {
        let bits = u64::from(self.body == Body::Moon) << 63
            | (self.sector.index() as u64) << 59
            | u64::from(self.depth) << 52
            | u64::from(self.x) << 26
            | u64::from(self.y);
        PatchId(bits)
    }

    /// Share of a sector side this node spans.
    fn span(self) -> f64 {
        1.0 / f64::from(1u32 << self.depth)
    }

    /// Width on the ground, metres: a sector side is a quarter of a great circle.
    fn side_m(self) -> f64 {
        self.span() * self.body.radius_m() * core::f64::consts::FRAC_PI_2
    }

    fn children(self) -> [Node; 4] {
        let (depth, x, y) = (self.depth + 1, self.x * 2, self.y * 2);
        [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(i, j)| Node {
            body: self.body,
            sector: self.sector,
            depth,
            x: x + i,
            y: y + j,
        })
    }

    /// The unit direction of a point of the node, `s` and `t` in `0..=1`.
    /// Values outside reach past the node, and past the sector: the projection
    /// stays continuous there, which is all the mesher needs for normals at
    /// the rim. Every body uses the planet's sector grid as its parametrisation.
    fn direction(self, s: f64, t: f64) -> [f64; 3] {
        let side = f64::from(SECTOR_SIDE) * self.span();
        let point = SurfacePoint::new(
            self.sector,
            (f64::from(self.x) + s) * side,
            (f64::from(self.y) + t) * side,
        );
        point.direction()
    }
}

/// What the streamer remembers about a built patch.
struct Built {
    center: DVec3,
    /// Bounding sphere radius around `center`, metres.
    radius_m: f64,
    last_used: u64,
    /// The chunk a body was in when this patch was meshed, if one was under
    /// it. A body that goes a chunk deeper asks for the patch again.
    under_chunk: Option<i64>,
}

pub struct Terrain {
    body: Body,
    built: HashMap<Node, Built>,
    changes: Vec<TerrainChange>,
    frame: u64,
    casters: Vec<PatchId>,
}

impl Terrain {
    pub fn new(body: Body) -> Terrain {
        Terrain {
            body,
            built: HashMap::new(),
            changes: Vec::new(),
            frame: 0,
            casters: Vec::new(),
        }
    }

    pub fn shadow_patches(&self) -> &[PatchId] {
        &self.casters
    }

    /// Forgets everything: the recipe changed.
    pub fn clear(&mut self) {
        for node in self.built.keys() {
            self.changes.push(TerrainChange::Remove(node.id()));
        }
        self.built.clear();
        self.casters.clear();
    }

    /// Mesh uploads and removals since the last call, in order.
    pub fn drain_changes(&mut self) -> Vec<TerrainChange> {
        core::mem::take(&mut self.changes)
    }

    /// Picks the patches to draw from `camera`, building missing ones within
    /// the per update budget. The camera is relative to the body's centre;
    /// `aspect` is width over height.
    pub fn update(&mut self, generator: &Generator, camera: &Camera, aspect: f32) -> Vec<PatchId> {
        self.select(generator, camera, aspect, BUILD_BUDGET)
    }

    /// Like [`Terrain::update`], but builds until nothing is missing. For
    /// headless renders, where the first frame is the only frame.
    pub fn settle(&mut self, generator: &Generator, camera: &Camera, aspect: f32) -> Vec<PatchId> {
        loop {
            let before = self.built.len();
            let patches = self.select(generator, camera, aspect, usize::MAX);
            if self.built.len() == before {
                return patches;
            }
        }
    }

    fn select(
        &mut self,
        generator: &Generator,
        camera: &Camera,
        aspect: f32,
        budget: usize,
    ) -> Vec<PatchId> {
        self.frame += 1;
        self.casters.clear();
        let under = Under::of(self.body, generator, camera);
        let mut draw = Vec::new();
        let mut missing: Vec<(f64, Node)> = Vec::new();
        // Patches that exist but stop above the body standing in them. They
        // go first: the ground under somebody's feet beats any new patch.
        let mut deepen: Vec<Node> = Vec::new();

        let body = self.body;
        let mut stack: Vec<Node> = Sector::ALL
            .into_iter()
            .map(|sector| Node {
                body,
                sector,
                depth: 0,
                x: 0,
                y: 0,
            })
            .collect();
        while let Some(node) = stack.pop() {
            let Some(built) = self.built.get_mut(&node) else {
                // Only roots get here unbuilt: children are visited once ready.
                missing.push((0.0, node));
                continue;
            };
            built.last_used = self.frame;
            let (center, radius_m) = (built.center, built.radius_m);
            // Somebody went a chunk deeper than this patch was meshed for:
            // it is missing the ground they are standing on, so it is built
            // again. Measured by the chunk, so walking about never remeshes.
            let shallow = |built: &Built, under: &Under| {
                built.under_chunk.is_none_or(|had| had > under.chunk())
            };
            if is_volume(node)
                && under.is_some_and(|under| {
                    under.reaches(node).is_some() && shallow(built, &under)
                })
            {
                deepen.push(node);
            }

            let distance = (center - camera.position).length();
            if node.depth < body.max_depth() && distance < node.side_m() * SPLIT_DISTANCE {
                let children = node.children();
                let mut ready = true;
                for child in children {
                    match self.built.get_mut(&child) {
                        // Siblings of a drawn node must not age out under it.
                        Some(built) => built.last_used = self.frame,
                        None => {
                            ready = false;
                            missing.push((distance, child));
                        }
                    }
                }
                if ready {
                    stack.extend(children);
                    continue;
                }
            }
            if visible(body, camera, aspect, center, radius_m) {
                draw.push(node.id());
            }
        }

        // Coarse before fine, near before far: the picture sharpens evenly.
        missing.sort_by(|a, b| (a.1.depth, a.0).partial_cmp(&(b.1.depth, b.0)).unwrap());
        let mut spent = 0usize;
        for node in deepen.into_iter().chain(missing.into_iter().map(|(_, n)| n)) {
            if spent >= budget {
                break;
            }
            let under_m = under.and_then(|under| under.reaches(node));
            spent = spent.saturating_add(cost(node));
            let mesh = build(generator, node, under_m);
            // The sea counts: over deep water the ground is far below the surface
            // that is actually in view.
            let ground = mesh.vertices.iter().map(|v| v.position);
            let sea = mesh.water.iter().flatten().map(|v| v.position);
            let radius_m = ground
                .chain(sea)
                .map(|position| Vec3::from(position).length())
                .fold(0.0, f32::max);
            self.built.insert(
                node,
                Built {
                    center: mesh.origin,
                    radius_m: f64::from(radius_m),
                    last_used: self.frame,
                    under_chunk: under
                        .filter(|_| under_m.is_some())
                        .map(|under| under.chunk()),
                },
            );
            self.changes.push(TerrainChange::Add(node.id(), mesh));
        }
        self.select_shadow_lod(camera.position);
        self.evict();
        draw
    }

    /// Shadows reuse built ancestors, never schedule extra generation. Nearby
    /// contacts keep metre detail; distant terrain casts from a coarser mesh.
    fn select_shadow_lod(&mut self, eye: DVec3) {
        let mut stack: Vec<_> = Sector::ALL
            .into_iter()
            .map(|sector| Node {
                body: self.body,
                sector,
                depth: 0,
                x: 0,
                y: 0,
            })
            .collect();
        while let Some(node) = stack.pop() {
            let Some(built) = self.built.get(&node) else {
                continue;
            };
            let distance = (built.center - eye).length() - built.radius_m;
            let target = if distance < 250.0 {
                0.0
            } else if distance < 350.0 {
                4.0
            } else {
                16.0
            };
            let children = node.children();
            if node.depth < self.body.max_depth()
                && node.side_m() / f64::from(PATCH_GRID) > target
                && children.iter().all(|child| self.built.contains_key(child))
            {
                stack.extend(children);
            } else {
                self.casters.push(node.id());
                self.built
                    .get_mut(&node)
                    .expect("selected built node")
                    .last_used = self.frame;
            }
        }
    }

    /// The spacing of the finest mesh built so far over a direction from the
    /// body's centre: the footprint the ground is *drawn* with there. Collision
    /// uses the full terrain, and a coarse mesh can sit well above it (small
    /// craters are faded out of it), so whatever must stay above what is on
    /// screen, the camera and a flyer, asks for the ground at this footprint.
    pub fn drawn_footprint_m(&self, direction: DVec3) -> f64 {
        let point = SurfacePoint::from_direction(direction.to_array());
        let side = f64::from(SECTOR_SIDE);
        let (s, t) = (
            (point.u / side).clamp(0.0, 1.0 - 1e-12),
            (point.v / side).clamp(0.0, 1.0 - 1e-12),
        );
        let mut finest = Node {
            body: self.body,
            sector: point.sector,
            depth: 0,
            x: 0,
            y: 0,
        };
        for depth in 1..=self.body.max_depth() {
            let cells = f64::from(1u32 << depth);
            let node = Node {
                body: self.body,
                sector: point.sector,
                depth,
                x: (s * cells) as u32,
                y: (t * cells) as u32,
            };
            if !self.built.contains_key(&node) {
                break;
            }
            finest = node;
        }
        finest.side_m() / f64::from(G)
    }

    /// Drops the patches unused for the longest while the cache is over size.
    fn evict(&mut self) {
        if self.built.len() <= CACHE_PATCHES {
            return;
        }
        let mut idle: Vec<(u64, Node)> = self
            .built
            .iter()
            .filter(|(_, built)| built.last_used < self.frame)
            .map(|(node, built)| (built.last_used, *node))
            .collect();
        idle.sort_by_key(|(last_used, node)| (*last_used, core::cmp::Reverse(node.depth)));
        let excess = self.built.len() - CACHE_PATCHES;
        for (_, node) in idle.into_iter().take(excess) {
            self.built.remove(&node);
            self.changes.push(TerrainChange::Remove(node.id()));
        }
    }
}

/// Inside the view frustum and not behind the body's own horizon.
fn visible(body: Body, camera: &Camera, aspect: f32, center: DVec3, radius_m: f64) -> bool {
    // Horizon: compare angles at the body's centre. Ground never sits below
    // the deepest floor, so a sphere at that depth is the occluder.
    let eye = camera.position.length();
    let reach = center.length() + radius_m;
    let angle = (camera.position / eye)
        .dot(center.normalize())
        .clamp(-1.0, 1.0)
        .acos();
    let occluder = body.radius_m() - body.deepest_m();
    let horizon = (occluder / eye).min(1.0).acos() + (occluder / reach).min(1.0).acos();
    if angle - radius_m / body.radius_m() > horizon {
        return false;
    }

    // Frustum: four side planes through the eye, in camera space.
    let p = camera.rotation.inverse() * (center - camera.position);
    let half_y = f64::from(camera.fov_y / 2.0);
    let half_x = (half_y.tan() * f64::from(aspect)).atan();
    let outside = |lateral: f64, half: f64| {
        // Signed distance to the plane tilted `half` from the view axis.
        lateral.abs() * half.cos() + p.z * half.sin() > radius_m
    };
    !(outside(p.x, half_x) || outside(p.y, half_y))
}

/// Where somebody is, when they are under the ground: the patch of the
/// deepest level that holds them, and their height over the datum.
///
/// This is the invoker of Voxel Plugin by another name. A patch nobody is
/// inside of is meshed for a camera looking at its surface; the patches
/// around a body have to hold the cave or the shaft it is standing in.
#[derive(Clone, Copy, Debug)]
struct Under {
    sector: Sector,
    patch: [i64; 2],
    height_m: f64,
}

impl Under {
    /// `None` over the ground, where the surface window already covers the
    /// eye, and on any body without a volume layer.
    fn of(body: Body, generator: &Generator, camera: &Camera) -> Option<Under> {
        if body != Body::Planet {
            return None;
        }
        let height_m = camera.position.length() - RADIUS_M;
        let point = SurfacePoint::from_direction(camera.position.to_array());
        let ground_m = generator.sample(point.direction()).height_m;
        if height_m > ground_m {
            return None;
        }
        let side = crate::volume::PATCH_BLOCKS;
        Some(Under {
            sector: point.sector,
            patch: [
                libm::floor(point.u / side as f64) as i64,
                libm::floor(point.v / side as f64) as i64,
            ],
            height_m,
        })
    }

    /// The height a node has to reach down to, if this body is near enough to
    /// it. A cave is wider than one patch and a body sees its walls, so this
    /// is a ring and not a square of one.
    fn reaches(self, node: Node) -> Option<f64> {
        let near = node.sector == self.sector
            && (i64::from(node.x) - self.patch[0]).abs() <= UNDER_RING
            && (i64::from(node.y) - self.patch[1]).abs() <= UNDER_RING;
        near.then_some(self.height_m)
    }

    /// Which chunk of the stack the body is in: what a rebuild is measured
    /// against, so walking about does not remesh, and descending does.
    fn chunk(self) -> i64 {
        libm::floor(self.height_m / (CHUNK_SIDE as f64 * BLOCK_M)) as i64
    }
}

/// What building one node costs, in units of a heightfield patch. A patch
/// that reaches down to a body costs the same as one that does not: it may
/// look further, but it meshes no more chunks (`volume::MOST_CHUNKS`).
fn cost(node: Node) -> usize {
    if is_volume(node) { VOLUME_COST } else { 1 }
}

/// Whether this node's ground is meshed from its density rather than its
/// height. Only the deepest level, and only where a cell is a block.
fn is_volume(node: Node) -> bool {
    node.body == Body::Planet && node.depth == node.body.max_depth()
}

/// Meshes one node. `under_m` is the height of a body inside it, which a
/// volume patch has to reach down to.
fn build(generator: &Generator, node: Node, under_m: Option<f64>) -> TerrainMesh {
    let body = node.body;
    let radius = body.radius_m();
    // One ring of samples past the rim, so rim normals match the neighbours'.
    let stride = (G + 3) as usize;
    let spacing_m = node.side_m() / f64::from(G);
    let mut samples: Vec<(DVec3, Sample)> = Vec::with_capacity(stride * stride);
    for j in -1..=G + 1 {
        for i in -1..=G + 1 {
            let direction =
                node.direction(f64::from(i) / f64::from(G), f64::from(j) / f64::from(G));
            let sample = body.sample(generator, direction, spacing_m);
            samples.push((DVec3::from(direction) * (radius + sample.height_m), sample));
        }
    }
    let at = |i: i32, j: i32| &samples[(j + 1) as usize * stride + (i + 1) as usize];

    let origin = at(G / 2, G / 2).0;
    let mut vertices = Vec::with_capacity(PATCH_VERTICES as usize);
    for j in 0..=G {
        for i in 0..=G {
            let (position, sample) = *at(i, j);
            let along_u = at(i + 1, j).0 - at(i - 1, j).0;
            let along_v = at(i, j + 1).0 - at(i, j - 1).0;
            let normal = along_u.cross(along_v).normalize();
            // A vertex under the shore wears the cover of the highest land
            // beside it, so no beach bleeds uphill across a coarse triangle.
            let cover = if shore(sample) {
                let around = (-1..=1).flat_map(|dj| (-1..=1).map(move |di| (di, dj)));
                around
                    .map(|(di, dj)| at(i + di, j + dj).1)
                    .filter(|beside| !shore(*beside))
                    .max_by(|a, b| a.height_m.total_cmp(&b.height_m))
                    .unwrap_or(sample)
            } else {
                sample
            };
            vertices.push(TerrainVertex {
                position: (position - origin).as_vec3().to_array(),
                normal: normal.as_vec3().to_array(),
                color: color(cover.material),
            });
        }
    }

    // Skirts hide the cracks between neighbours of different depth.
    let drop_m = spacing_m * 2.0;
    let runs = [
        (0..=G).map(|k| (k, 0)).collect::<Vec<_>>(),
        (0..=G).map(|k| (k, G)).collect(),
        (0..=G).map(|k| (0, k)).collect(),
        (0..=G).map(|k| (G, k)).collect(),
    ];
    for &(i, j) in runs.iter().flatten() {
        let mut vertex = vertices[(j * (G + 1) + i) as usize];
        let down = -(at(i, j).0.normalize() * drop_m).as_vec3();
        vertex.position = (Vec3::from(vertex.position) + down).to_array();
        vertices.push(vertex);
    }

    // The sea over this patch: the same grid flattened to sea level. The ring
    // counts too, so a neighbour's shoreline never leaves a gap at the rim.
    let wet = body.has_sea() && samples.iter().any(|(_, sample)| sample.height_m < 0.0);
    let water = wet.then(|| {
        let sea = |(i, j): (i32, i32)| {
            let (position, sample) = at(i, j);
            WaterVertex {
                position: (position.normalize() * radius - origin)
                    .as_vec3()
                    .to_array(),
                depth_m: -sample.height_m as f32,
            }
        };
        // The sea is see-through, so a lowered skirt would show through the
        // neighbour's surface. The sphere is smooth enough to need none: the
        // skirt run collapses onto the rim.
        let grid = (0..=G).flat_map(|j| (0..=G).map(move |i| (i, j)));
        grid.chain(runs.iter().flatten().copied())
            .map(sea)
            .collect()
    });
    let grass = if body == Body::Planet {
        super::grass::build(
            super::grass::Patch {
                seed: generator.recipe().seed,
                sector: node.sector.index() as u32,
                cell: [node.x, node.y],
                depth: node.depth,
            },
            origin,
            &vertices,
            &samples,
        )
    } else {
        Vec::new()
    };
    // The deepest level of the quadtree meshes the ground from its density
    // instead of its height, so a cave, an arch and an overhang can exist at
    // all. The level above is still a heightfield and still carries a skirt,
    // and the two agree on where the ground is, so the boundary between them
    // is the one the LOD already had.
    let volume = is_volume(node)
        .then(|| {
            let side = i64::from(SECTOR_SIDE >> node.depth);
            debug_assert_eq!(side, crate::volume::PATCH_BLOCKS);
            crate::volume::build(
                generator,
                node.sector,
                [i64::from(node.x) * side, i64::from(node.y) * side],
                radius,
                origin,
                color,
                under_m,
            )
        })
        .filter(|(_, indices)| !indices.is_empty());
    let (vertices, indices) = volume.unwrap_or((vertices, Vec::new()));

    TerrainMesh {
        grass,
        origin,
        vertices,
        indices,
        water,
    }
}

/// Ground the renderer paints by height over the sea, never by vertex.
fn shore(sample: Sample) -> bool {
    matches!(
        sample.material,
        Material::Sand | Material::Seabed | Material::Water
    )
}

/// Albedo and gloss of the ground cover. Rock on slopes and the shore (sand,
/// sea floor) are the renderer's job: it sees the slope and the height per
/// pixel, the same at every LOD. A shore color here only shows where no land
/// is near, under the renderer's own.
fn color(material: Material) -> [u8; 4] {
    match material {
        Material::Snow => [236, 238, 240, 70],
        Material::Sand => [206, 192, 150, 8],
        Material::Grass => [104, 138, 70, 0],
        // A shade off the meadow: open ground that waits for its trees, no tufts.
        Material::Forest => [92, 128, 66, 0],
        Material::Rock => [118, 112, 106, 12],
        // v1 calls everything under the sea water; it is sea floor all the same.
        Material::Seabed | Material::Water => [112, 116, 98, 0],
        Material::Regolith => [112, 110, 105, 0],
    }
}
