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
//! dips under a sea it also carries a water surface. Nature is not editable
//! (DECISIONS 58): a stamp under a volume changes the ground the generator
//! gives, and the patches over it are built again (`Terrain::reshape`).
//!
//! Everything here is relative to the centre of the body: the planet sits at
//! the world origin, the moon moves, and whoever draws a patch adds the body's
//! centre for that frame.

use std::collections::{HashMap, HashSet};

use glam::{DVec3, Vec3};
use scene::{
    Camera, PATCH_GRID, PATCH_VERTICES, PatchId, TerrainChange, TerrainMesh, TerrainVertex,
    WaterVertex,
};
use topology::{Grid, QuadSphere, Sector, SurfacePoint};
use worldgen::{Generator, MOON_RADIUS_M, Material, Sample};

/// A node splits when the camera is closer than this many node widths.
const SPLIT_DISTANCE: f64 = 2.4;
/// Patches generated per update. Each costs about a millisecond or two.
const BUILDS_PER_UPDATE: usize = 6;
/// Patches kept before the least recently used ones are dropped.
const CACHE_PATCHES: usize = 1400;

const G: i32 = PATCH_GRID as i32;

/// The body a terrain streamer covers. Every body is a quad sphere meshed by
/// the same quadtree; they differ in size, in depth, and in having a sea.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    Planet,
    Moon,
}

/// The body a terrain streamer covers: a kind, and the quad sphere it is
/// printed at. Every body is meshed by the same quadtree; they differ in size,
/// in depth, and in having a sea.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Body {
    kind: Kind,
    sphere: QuadSphere,
}

impl Body {
    pub fn new(kind: Kind, sphere: QuadSphere) -> Body {
        Body { kind, sphere }
    }

    /// The grid a body is parametrised on. Seams, neighbours and directions
    /// are pure arithmetic and belong to the grid, not to the metres (49).
    fn grid(self) -> Grid {
        self.sphere.blocks()
    }

    /// Metres of this body per metre of the reference body. Heights are
    /// written once in reference metres and printed at a world's own size
    /// (50), so anything this file measures in metres scales with it.
    fn scale(self) -> f64 {
        f64::from(self.grid().side()) / f64::from(1u32 << topology::MAX_BITS)
    }

    pub fn radius_m(self) -> f64 {
        match self.kind {
            Kind::Planet => self.sphere.radius_m(),
            Kind::Moon => MOON_RADIUS_M,
        }
    }

    /// Deepest quadtree level. The planet: patches of 32 blocks, one vertex
    /// per block, which is five bits under the sector side. The moon is
    /// smaller, so it gets as fine sooner. A body with no room for a whole
    /// patch stops at its root, which is what the saturating subtraction says.
    fn max_depth(self) -> u32 {
        match self.kind {
            Kind::Planet => self.sphere.bits().saturating_sub(5),
            Kind::Moon => 9,
        }
    }

    fn has_sea(self) -> bool {
        self.kind == Kind::Planet
    }

    /// No ground lies deeper than this under the datum, whatever the recipe.
    fn deepest_m(self) -> f64 {
        match self.kind {
            Kind::Planet => 2000.0 * self.scale(),
            Kind::Moon => 1500.0,
        }
    }

    pub fn sample(self, generator: &Generator, direction: [f64; 3], footprint_m: f64) -> Sample {
        match self.kind {
            Kind::Planet => generator.sample_at(direction, footprint_m),
            Kind::Moon => generator.moon_sample_at(direction, footprint_m),
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
        let bits = u64::from(self.body.kind == Kind::Moon) << 63
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
        let grid = self.body.grid();
        let side = f64::from(grid.side()) * self.span();
        let point = SurfacePoint::new(
            self.sector,
            (f64::from(self.x) + s) * side,
            (f64::from(self.y) + t) * side,
        );
        grid.direction(point)
    }
}

/// What the streamer remembers about a built patch.
struct Built {
    center: DVec3,
    /// Bounding sphere radius around `center`, metres.
    radius_m: f64,
    last_used: u64,
}

pub struct Terrain {
    body: Body,
    built: HashMap<Node, Built>,
    /// Built patches whose ground has changed under them: each keeps being
    /// drawn until the one that replaces it is built.
    stale: HashSet<Node>,
    changes: Vec<TerrainChange>,
    frame: u64,
    casters: Vec<PatchId>,
    /// Whether the last update found nothing to build.
    settled: bool,
}

impl Terrain {
    pub fn new(body: Body) -> Terrain {
        Terrain {
            body,
            built: HashMap::new(),
            stale: HashSet::new(),
            changes: Vec::new(),
            frame: 0,
            casters: Vec::new(),
            settled: false,
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
        self.stale.clear();
        self.casters.clear();
    }

    /// The ground changed within `angle` radians of `middle` (unit, from the
    /// body's centre): every patch that reaches there is built again. Nothing
    /// comes down first, so there is never a hole.
    pub fn reshape(&mut self, middle: DVec3, angle: f64) {
        let radius_m = self.body.radius_m();
        for (node, built) in &self.built {
            let reach = (angle + built.radius_m / radius_m).min(core::f64::consts::PI);
            if built.center.normalize().dot(middle) >= reach.cos() {
                self.stale.insert(*node);
            }
        }
    }

    /// Mesh uploads and removals since the last call, in order.
    pub fn drain_changes(&mut self) -> Vec<TerrainChange> {
        core::mem::take(&mut self.changes)
    }

    /// Picks the patches to draw from `camera`, building missing ones within
    /// the per update budget. The camera is relative to the body's centre;
    /// `aspect` is width over height.
    pub fn update(&mut self, generator: &Generator, camera: &Camera, aspect: f32) -> Vec<PatchId> {
        self.select(generator, camera, aspect, BUILDS_PER_UPDATE)
    }

    /// Like [`Terrain::update`], but builds until nothing is missing. For
    /// headless renders, where the first frame is the only frame.
    pub fn settle(&mut self, generator: &Generator, camera: &Camera, aspect: f32) -> Vec<PatchId> {
        loop {
            let patches = self.select(generator, camera, aspect, usize::MAX);
            if self.settled {
                return patches;
            }
        }
    }

    /// Whether the last update found nothing to build: what is drawn is the
    /// view at the detail it is meant to have. A step, a leap or a new recipe
    /// unsettle it until the streamer catches up.
    pub fn settled(&self) -> bool {
        self.settled
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
        let mut draw = Vec::new();
        let mut missing: Vec<(f64, Node)> = Vec::new();

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

            let distance = (center - camera.position).length();
            if self.stale.contains(&node) {
                missing.push((distance, node));
            }
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

        self.settled = missing.is_empty();
        // Coarse before fine, near before far: the picture sharpens evenly.
        missing.sort_by(|a, b| (a.1.depth, a.0).partial_cmp(&(b.1.depth, b.0)).unwrap());
        for (_, node) in missing.into_iter().take(budget) {
            let mesh = build(generator, node);
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
                },
            );
            self.stale.remove(&node);
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
                // Ground coarser than a shadow texel cannot be in a shadow map
                // at all (DECISIONS 55): one of its triangles spans hundreds of
                // texels and its depth varies across a single one by more than
                // any bias can lift, so every one of them fails its own depth
                // test and turns black. Nothing is lost by leaving it out,
                // because ground seen from that far is lit by the angle of the
                // sun, which is what lights a body from space anyway.
                if node.side_m() / f64::from(PATCH_GRID) > scene::SHADOW_CASTER_CELL_M {
                    continue;
                }
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
        let grid = self.body.grid();
        let point = grid.surface_point(direction.to_array());
        let side = f64::from(grid.side());
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
            self.stale.remove(&node);
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

/// Meshes one node.
fn build(generator: &Generator, node: Node) -> TerrainMesh {
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
                color: color(cover),
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
    let grass = if body.kind == Kind::Planet {
        super::grass::build(
            super::grass::Patch {
                seed: generator.recipe().seed,
                sector: node.sector.index() as u32,
                cell: [node.x, node.y],
                depth: node.depth,
                sector_side: body.grid().side(),
            },
            origin,
            &vertices,
            &samples,
        )
    } else {
        Vec::new()
    };
    TerrainMesh {
        grass,
        origin,
        vertices,
        // A heightfield patch is a regular grid, so it shares one index buffer
        // with every other patch and carries none of its own.
        indices: Vec::new(),
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
fn color(sample: Sample) -> [u8; 4] {
    match sample.material {
        Material::Snow => [236, 238, 240, 70],
        Material::Sand => [206, 192, 150, 8],
        Material::Grass => [104, 138, 70, 0],
        // A shade off the meadow: open ground that waits for its trees, no tufts.
        Material::Forest => [92, 128, 66, 0],
        Material::Rock => [118, 112, 106, 12],
        // v1 calls everything under the sea water; it is sea floor all the same.
        Material::Seabed | Material::Water => [112, 116, 98, 0],
        Material::Regolith => [112, 110, 105, 0],
        // Worked earth under a volume: bare, a little warmer than rock.
        Material::Plot => [128, 112, 90, 0],
    }
}
