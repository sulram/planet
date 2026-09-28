//! The build layer on the client: volumes seated on the planet, the tool that
//! strokes them, and the meshes that draw them.
//!
//! A volume knows no sphere (`voxel`). Here it is seated: its cell
//! `(x, y, z)` is the address `(u0 + x, v0 + y, h0 + z)` of one sector, so a
//! cube is exactly a block of the world, and a body walks on it in address
//! space the way it walks on the ground (CLAUDE.md, simulate flat, render
//! spherical). Each corner of each side is bent onto the planet on its own,
//! which is why the sides are never merged.
//!
//! Volumes live here only: nothing is sent or kept yet (DECISIONS 75).

use std::collections::BTreeSet;

use glam::{DQuat, DVec3, Vec3};
use scene::{VolumeChange, VolumeMesh, VolumeMeshId, VolumeVertex};
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint};
use voxel::{Cell, Gesture, Hit, Quad, Span, Volume, crossing};
use worldgen::{Generator, Stamp};

use crate::collision::{Footing, STEP_M};
use crate::seam::{BuildRefusal, Tool};

/// Cells along each side of a volume opened where someone stands: 32 metres
/// at the middle of a sector, as tall as it is wide.
const SIDE: u32 = 64;
/// Metres over which the ground around a volume eases back to its own.
const BLEND_M: f64 = 8.0;
/// Blocks kept between a volume and the edge of its sector, or another
/// volume: room for its margin even where blocks are smallest.
const MARGIN: i64 = 32;
/// How far a pointer reaches into the world to build, metres.
const REACH_M: f64 = 80.0;
/// Metres between the points of a line of sight bent into a volume's cells:
/// a block, straight enough to walk exactly between them.
const SIGHT_STEP_M: f64 = BLOCK_M;
/// Half the width of a standing body in address space, in cells: how near a
/// wall it comes.
const BODY_HALF: f64 = 0.6;
/// How far the ghost of a stroke stands off the cubes it covers, metres.
const GHOST_LIFT_M: f32 = 0.012;
/// The ghost's id, which no chunk of any volume has.
const GHOST: VolumeMeshId = VolumeMeshId(u64::MAX);
/// Strokes kept to take back.
const HISTORY: usize = 100;

/// The eye a tool aims from: where the camera is, how it looks, and where
/// the pointer is on the view.
#[derive(Clone, Copy, Debug)]
pub struct Eye {
    pub position: DVec3,
    /// World from camera. The camera looks down its `-Z`, `+Y` is up.
    pub rotation: DQuat,
    /// Vertical field of view, radians.
    pub fov_y: f64,
    /// Width over height.
    pub aspect: f64,
    /// Fractions of the view from the top left.
    pub pointer: [f64; 2],
}

impl Eye {
    /// The pointer on the view in units of half its height, `y` up: a
    /// distance there is the same across and down.
    fn at(&self) -> [f64; 2] {
        let [x, y] = self.pointer;
        [(2.0 * x - 1.0) * self.aspect, 1.0 - 2.0 * y]
    }

    /// Where the eye is and which way the pointer looks from it, unit.
    pub fn sight(&self) -> (DVec3, DVec3) {
        let [x, y] = self.at();
        let tan = (self.fov_y / 2.0).tan();
        let look = DVec3::new(x * tan, y * tan, -1.0).normalize();
        (self.position, self.rotation * look)
    }
}

/// The paints a cell can take, as sRGB: whites and greys, earth, green, water
/// and a few loud ones.
pub const PALETTE: [[u8; 3]; 16] = [
    [242, 240, 235],
    [184, 184, 179],
    [92, 92, 90],
    [30, 30, 31],
    [156, 59, 46],
    [200, 105, 63],
    [217, 192, 140],
    [122, 82, 48],
    [95, 122, 58],
    [63, 143, 74],
    [47, 140, 140],
    [90, 167, 224],
    [42, 63, 122],
    [124, 82, 184],
    [232, 122, 168],
    [240, 201, 58],
];

/// What a stroke that deletes shows over what it takes away.
const DELETE_COLOR: [u8; 3] = [230, 70, 60];

/// A volume and where it is seated.
struct Seated {
    id: u32,
    volume: Volume,
    sector: Sector,
    /// The address of cell `(0, 0, 0)`: `u`, `v` and `h`, in blocks.
    origin: [i64; 3],
}

impl Seated {
    /// Whether a column of the sector is on this volume's footprint, or
    /// within `margin` blocks of it.
    fn near(&self, sector: Sector, u: f64, v: f64, margin: f64) -> bool {
        let side = f64::from(SIDE);
        let (du, dv) = (u - self.origin[0] as f64, v - self.origin[1] as f64);
        sector == self.sector
            && (-margin..side + margin).contains(&du)
            && (-margin..side + margin).contains(&dv)
    }

    /// Where a lattice point of this volume is in the world.
    fn corner(&self, sphere: QuadSphere, p: [i32; 3]) -> DVec3 {
        let point = SurfacePoint::new(
            self.sector,
            (self.origin[0] + i64::from(p[0])) as f64,
            (self.origin[1] + i64::from(p[1])) as f64,
        );
        let height_m = (self.origin[2] + i64::from(p[2])) as f64 * BLOCK_M;
        DVec3::from(sphere.position(point, height_m))
    }

    /// A point of the world in this volume's cells, when it is over this
    /// volume's sector.
    fn cells_of(&self, sphere: QuadSphere, p: DVec3) -> Option<[f64; 3]> {
        let point = sphere.blocks().surface_point(p.to_array());
        (point.sector == self.sector).then(|| {
            [
                point.u - self.origin[0] as f64,
                point.v - self.origin[1] as f64,
                (p.length() - sphere.radius_m()) / BLOCK_M - self.origin[2] as f64,
            ]
        })
    }

    /// The middle of the box in the world, and a radius that holds all of it.
    fn bounds(&self, sphere: QuadSphere) -> (DVec3, f64) {
        let size = self.volume.size().map(|n| n as i32);
        let middle = self.corner(sphere, size.map(|n| n / 2));
        let radius_m = self.corner(sphere, [0; 3]).distance(middle) * 1.5;
        (middle, radius_m)
    }

    /// A line of sight from `from` along `toward` (unit), as a path in this
    /// volume's cells: the part of it that could reach the box.
    fn sight(&self, sphere: QuadSphere, from: DVec3, toward: DVec3) -> Vec<[f64; 3]> {
        let (middle, radius_m) = self.bounds(sphere);
        let along = (middle - from).dot(toward);
        let miss2 = (middle - from).length_squared() - along * along;
        if miss2 > radius_m * radius_m {
            return Vec::new();
        }
        let half = (radius_m * radius_m - miss2).sqrt();
        let (near, far) = ((along - half).max(0.0), (along + half).min(REACH_M));
        let steps = ((far - near) / SIGHT_STEP_M).ceil().max(0.0) as usize;
        (0..=steps)
            .filter_map(|i| {
                let t = (near + i as f64 * SIGHT_STEP_M).min(far);
                self.cells_of(sphere, from + toward * t)
            })
            .collect()
    }

    fn mesh_id(&self, chunk: [u32; 3]) -> VolumeMeshId {
        let [cx, cy, _] = self.volume.chunk_count();
        let index = (chunk[2] * cy + chunk[1]) * cx + chunk[0];
        VolumeMeshId(u64::from(self.id) << 32 | u64::from(index))
    }
}

/// What the pointer is over.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Aim {
    site: usize,
    hit: Hit,
}

/// A stroke being drawn: from the cell it started on, across a layer of cells
/// through it, to where the pointer meets that layer.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Stroke {
    site: usize,
    tool: Tool,
    start: [i32; 3],
    /// The axis the layer is across, and where along it the pointer is read.
    /// A slab lies on the side the stroke started on and is read on that
    /// side; a wall stands up from it, across another axis, and is read on
    /// its face toward the eye. Read anywhere else, the cell under the
    /// pointer is not the one the stroke shows.
    across: (usize, f64),
    end: [i32; 3],
}

impl Stroke {
    fn span(self) -> Span {
        Span::between(self.start, self.end)
    }

    fn gesture(self, paint: u8) -> Gesture {
        let span = self.span();
        match self.tool {
            Tool::Create => Gesture::Create { span, paint },
            Tool::Delete => Gesture::Delete { span },
            Tool::Paint => Gesture::Paint { span, paint },
        }
    }
}

/// A stroke that landed, as the cells of its box before and after.
struct Change {
    site: usize,
    span: Span,
    before: Vec<Cell>,
    after: Vec<Cell>,
}

/// Every volume, the tool in hand and what it is drawing.
pub struct Build {
    sites: Vec<Seated>,
    next_id: u32,
    tool: Option<Tool>,
    /// The tool taken last, which starting to build again takes.
    last_tool: Tool,
    paint: u8,
    aim: Option<Aim>,
    stroke: Option<Stroke>,
    using: bool,
    /// What the ghost shows now: it is meshed again only when that changes.
    ghost: Option<Gesture>,
    drawn: BTreeSet<VolumeMeshId>,
    changes: Vec<VolumeChange>,
    /// Strokes to take back, the last one last, and those taken back.
    done: Vec<Change>,
    undone: Vec<Change>,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            sites: Vec::new(),
            next_id: 0,
            tool: None,
            last_tool: Tool::Create,
            paint: 0,
            aim: None,
            stroke: None,
            using: false,
            ghost: None,
            drawn: BTreeSet::new(),
            changes: Vec::new(),
            done: Vec::new(),
            undone: Vec::new(),
        }
    }
}

impl Build {
    pub fn tool(&self) -> Option<Tool> {
        self.tool
    }

    pub fn paint(&self) -> u8 {
        self.paint
    }

    /// The tool that starting to build again takes.
    pub fn last_tool(&self) -> Tool {
        self.last_tool
    }

    /// Takes a tool or puts it down. A stroke half drawn is dropped.
    pub fn take(&mut self, tool: Option<Tool>) {
        self.tool = tool;
        if let Some(tool) = tool {
            self.last_tool = tool;
        }
        self.stroke = None;
        self.aim = None;
    }

    pub fn set_paint(&mut self, paint: u8) {
        self.paint = paint.min(PALETTE.len() as u8 - 1);
    }

    /// Whether a volume stands on a column.
    pub fn covers(&self, point: SurfacePoint) -> bool {
        self.sites
            .iter()
            .any(|site| site.near(point.sector, point.u, point.v, 0.0))
    }

    /// Opens a volume around a column, on ground held flat at its height, and
    /// returns the stamp that holds it: the ground under it has to be drawn
    /// again.
    pub fn open(
        &mut self,
        generator: &mut Generator,
        point: SurfacePoint,
    ) -> Result<Stamp, BuildRefusal> {
        let sphere = generator.sphere();
        let side = i64::from(sphere.blocks().side());
        let half = i64::from(SIDE / 2);
        let (u0, v0) = (point.u as i64 - half, point.v as i64 - half);
        let fits = |low: i64| low >= MARGIN && low + i64::from(SIDE) + MARGIN <= side;
        if !fits(u0) || !fits(v0) {
            return Err(BuildRefusal::Seam);
        }
        // The new footprint reaches `half` past the point, and keeps a margin
        // from any other one.
        let reach = (half + MARGIN) as f64;
        if self
            .sites
            .iter()
            .any(|site| site.near(point.sector, point.u, point.v, reach))
        {
            return Err(BuildRefusal::Neighbour);
        }
        let ground_m = generator.sample(sphere.blocks().direction(point)).height_m;
        if ground_m < 0.0 {
            return Err(BuildRefusal::Sea);
        }
        let h0 = libm::round(ground_m / BLOCK_M) as i64;
        let stamp = Stamp::new(
            sphere,
            point.sector,
            [u0 as f64, (u0 + i64::from(SIDE)) as f64],
            [v0 as f64, (v0 + i64::from(SIDE)) as f64],
            h0 as f64 * BLOCK_M,
            BLEND_M,
        );
        generator.stamp(stamp);
        self.sites.push(Seated {
            id: self.next_id,
            volume: Volume::new([SIDE; 3]),
            sector: point.sector,
            origin: [u0, v0, h0],
        });
        self.next_id += 1;
        Ok(stamp)
    }

    /// Forgets every volume: another world.
    pub fn clear(&mut self) {
        for id in core::mem::take(&mut self.drawn) {
            self.changes.push(VolumeChange::Remove(id));
        }
        if self.ghost.take().is_some() {
            self.changes.push(VolumeChange::Remove(GHOST));
        }
        self.sites.clear();
        self.aim = None;
        self.stroke = None;
        self.done.clear();
        self.undone.clear();
    }

    /// Drops the stroke being drawn, if there is one. True when there was.
    pub fn cancel(&mut self) -> bool {
        self.stroke.take().is_some()
    }

    /// Whether there is a stroke to take back, and one to put back.
    pub fn history(&self) -> (bool, bool) {
        (!self.done.is_empty(), !self.undone.is_empty())
    }

    /// Takes back the last stroke that landed.
    pub fn undo(&mut self, sphere: QuadSphere) {
        if let Some(change) = self.done.pop() {
            self.stroke = None;
            let changed = self.sites[change.site]
                .volume
                .restore(change.span, &change.before);
            if let Some(changed) = changed {
                self.redraw(sphere, change.site, changed);
            }
            self.undone.push(change);
        }
    }

    /// Puts back the last stroke taken back.
    pub fn redo(&mut self, sphere: QuadSphere) {
        if let Some(change) = self.undone.pop() {
            self.stroke = None;
            let changed = self.sites[change.site]
                .volume
                .restore(change.span, &change.after);
            if let Some(changed) = changed {
                self.redraw(sphere, change.site, changed);
            }
            self.done.push(change);
        }
    }

    /// One frame of the tool: what the line of sight through the pointer
    /// meets, the stroke while the button is held, and the gesture when it
    /// is let go. A stroke started with `upright` held stands up as a wall.
    pub fn update(&mut self, sphere: QuadSphere, eye: &Eye, using: bool, upright: bool) {
        let Some(tool) = self.tool else {
            self.using = using;
            self.show_ghost(sphere, None);
            return;
        };
        let (from, toward) = eye.sight();
        let paths: Vec<Vec<[f64; 3]>> = self
            .sites
            .iter()
            .map(|site| site.sight(sphere, from, toward))
            .collect();
        self.aim = self.aim_at(sphere, from, &paths);

        let pressed = using && !self.using;
        let released = !using && self.using;
        self.using = using;
        if pressed {
            self.stroke = self.aim.and_then(|aim| {
                let mut stroke = self.start(tool, aim)?;
                if upright {
                    let site = &self.sites[stroke.site];
                    stroke.across = standing(site, sphere, eye, stroke.start, stroke.across.0);
                }
                Some(stroke)
            });
        }
        if let Some(stroke) = &mut self.stroke {
            let site = &self.sites[stroke.site];
            let (across, at) = stroke.across;
            // On the layer the stroke goes in, whatever is behind it.
            if let Some(p) = crossing(&paths[stroke.site], across, at) {
                let bounds = site.volume.bounds();
                let mut end =
                    [0, 1, 2].map(|i| (p[i].floor() as i32).clamp(bounds.min[i], bounds.max[i]));
                end[across] = stroke.start[across];
                stroke.end = end;
            }
        }
        if released && let Some(stroke) = self.stroke.take() {
            self.apply(sphere, stroke.site, stroke.gesture(self.paint));
        }

        let preview = match (self.stroke, self.aim) {
            (Some(stroke), _) => Some((stroke.site, stroke.gesture(self.paint))),
            (None, Some(aim)) => self
                .start(tool, aim)
                .map(|stroke| (stroke.site, stroke.gesture(self.paint))),
            (None, None) => None,
        };
        self.show_ghost(sphere, preview);
    }

    /// The nearest thing any volume's line of sight meets.
    fn aim_at(&self, sphere: QuadSphere, from: DVec3, paths: &[Vec<[f64; 3]>]) -> Option<Aim> {
        paths
            .iter()
            .enumerate()
            .filter_map(|(site, path)| {
                let hit = self.sites[site].volume.trace(path)?;
                let middle = self.sites[site].corner(sphere, hit.cell);
                Some((middle.distance(from), Aim { site, hit }))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, aim)| aim)
    }

    /// The stroke a tool starts where it aims, if it can start there: a new
    /// cell goes in the air before the side hit, and what is taken away or
    /// repainted is a cell, never the floor.
    fn start(&self, tool: Tool, aim: Aim) -> Option<Stroke> {
        let cell = match tool {
            Tool::Create => aim.hit.before(),
            Tool::Delete | Tool::Paint if aim.hit.on_floor() => return None,
            Tool::Delete | Tool::Paint => aim.hit.cell,
        };
        let face = aim.hit.face;
        let surface = f64::from(aim.hit.cell[face.axis] + i32::from(face.positive));
        self.sites[aim.site]
            .volume
            .contains(cell)
            .then_some(Stroke {
                site: aim.site,
                tool,
                start: cell,
                across: (face.axis, surface),
                end: cell,
            })
    }

    /// Lays a gesture on the first volume, as a stroke would.
    #[cfg(test)]
    pub(crate) fn lay(&mut self, sphere: QuadSphere, gesture: Gesture) {
        self.apply(sphere, 0, gesture);
    }

    /// Applies a gesture, keeps it to take back, and draws what it changed.
    fn apply(&mut self, sphere: QuadSphere, site: usize, gesture: Gesture) {
        let volume = &mut self.sites[site].volume;
        let span = gesture.span();
        let before = volume.cells(span);
        let Some(changed) = volume.apply(gesture) else {
            return;
        };
        let after = volume.cells(span);
        self.done.push(Change {
            site,
            span,
            before,
            after,
        });
        if self.done.len() > HISTORY {
            self.done.remove(0);
        }
        self.undone.clear();
        self.redraw(sphere, site, changed);
    }

    /// Draws again every chunk whose sides a change to some cells could have
    /// changed: the cells, and one more all round, which is as far as a side
    /// or a corner looks.
    fn redraw(&mut self, sphere: QuadSphere, site: usize, changed: Span) {
        // The ghost showed what the cells were: it is meshed again from what
        // they are now.
        if self.ghost.take().is_some() {
            self.changes.push(VolumeChange::Remove(GHOST));
        }
        let seated = &self.sites[site];
        for chunk in seated.volume.chunks_in(changed.grown(1)) {
            let id = seated.mesh_id(chunk);
            let quads = seated.volume.faces(chunk);
            if quads.is_empty() {
                if self.drawn.remove(&id) {
                    self.changes.push(VolumeChange::Remove(id));
                }
                continue;
            }
            let mesh = mesh(
                seated.sector,
                seated.origin,
                sphere,
                &quads,
                paint_color,
                0.0,
            );
            self.drawn.insert(id);
            self.changes.push(VolumeChange::Add(id, mesh));
        }
    }

    /// Meshes the ghost of a gesture when it is not the one shown: exactly
    /// the cells it would change, so a stroke shows what it will do.
    fn show_ghost(&mut self, sphere: QuadSphere, preview: Option<(usize, Gesture)>) {
        let wanted = preview.map(|(_, gesture)| gesture);
        if wanted == self.ghost {
            return;
        }
        self.ghost = wanted;
        let Some((site, gesture)) = preview else {
            self.changes.push(VolumeChange::Remove(GHOST));
            return;
        };
        let seated = &self.sites[site];
        let span = gesture.span();
        // Creating takes air; deleting and painting take what is solid.
        let (color, takes_air) = match gesture {
            Gesture::Create { paint, .. } => (paint_color(paint), true),
            Gesture::Paint { paint, .. } => (paint_color(paint), false),
            Gesture::Delete { .. } => {
                let [r, g, b] = DELETE_COLOR;
                ([r, g, b, 0], false)
            }
        };
        let mut ghost = Volume::new(seated.volume.size());
        for at in span.cells().filter(|&at| seated.volume.contains(at)) {
            if seated.volume.get(at).is_air() == takes_air {
                ghost.apply(Gesture::Create {
                    span: Span::cell(at),
                    paint: 0,
                });
            }
        }
        let quads = span
            .meet(ghost.bounds())
            .map(|span| ghost.faces_in(span))
            .unwrap_or_default();
        let mut mesh = mesh(
            seated.sector,
            seated.origin,
            sphere,
            &quads,
            |_| color,
            GHOST_LIFT_M,
        );
        // A ghost is not shut in by itself.
        for vertex in &mut mesh.vertices {
            vertex.open = 1.0;
        }
        self.changes.push(VolumeChange::Add(GHOST, mesh));
    }

    /// What holds up a body standing at a point with its feet at `feet_m`,
    /// as far as the volumes go: `None` away from every volume.
    ///
    /// A body is as wide as [`BODY_HALF`] each way, so it stands on the
    /// highest cell under any part of it and stops short of a wall.
    pub fn footing(&self, point: SurfacePoint, feet_m: f64) -> Option<Footing> {
        let site = self
            .sites
            .iter()
            .find(|site| site.near(point.sector, point.u, point.v, BODY_HALF))?;
        let (x, y) = (
            point.u - site.origin[0] as f64,
            point.v - site.origin[1] as f64,
        );
        let reach = (feet_m + STEP_M) / BLOCK_M - site.origin[2] as f64;
        let columns = |at: f64| (at - BODY_HALF).floor() as i32..=(at + BODY_HALF).floor() as i32;
        let gaps = columns(x)
            .flat_map(|cx| columns(y).map(move |cy| (cx, cy)))
            .filter_map(|(cx, cy)| site.volume.gap(cx, cy, reach));
        let h0 = site.origin[2] as f64;
        let metres = |cells: i32| (h0 + f64::from(cells)) * BLOCK_M;
        gaps.map(|gap| Footing {
            floor_m: Some(metres(gap.floor)),
            ceiling_m: gap.ceiling.map(metres),
        })
        .reduce(Footing::with)
    }

    /// Mesh uploads and removals since the last call, in order.
    pub fn drain_changes(&mut self) -> Vec<VolumeChange> {
        core::mem::take(&mut self.changes)
    }

    /// Every volume mesh to draw.
    pub fn drawn(&self) -> Vec<VolumeMeshId> {
        self.drawn.iter().copied().collect()
    }

    /// The ghost of the stroke the tool would make, while there is one.
    pub fn ghost(&self) -> Option<VolumeMeshId> {
        self.ghost.map(|_| GHOST)
    }
}

/// The layer a stroke stands up in from the side it started on: across
/// whichever of the other two axes runs most into the view, so the wall
/// faces the eye as squarely as the grid allows, read on its face toward it.
fn standing(
    site: &Seated,
    sphere: QuadSphere,
    eye: &Eye,
    start: [i32; 3],
    facing: usize,
) -> (usize, f64) {
    let middle = |cell: [i32; 3]| {
        let low = site.corner(sphere, cell);
        let high = site.corner(sphere, cell.map(|n| n + 1));
        (low + high) / 2.0
    };
    let step = |axis: usize| {
        let mut next = start;
        next[axis] += 1;
        middle(next) - middle(start)
    };
    let (_, forward) = eye.sight();
    let depth = |axis: usize| step(axis).normalize_or_zero().dot(forward).abs();
    let [p, q] = [(facing + 1) % 3, (facing + 2) % 3];
    let across = if depth(p) >= depth(q) { p } else { q };
    // Its face toward the eye: the low side when the eye looks up the axis.
    let near = f64::from(start[across] + i32::from(step(across).dot(forward) < 0.0));
    (across, near)
}

/// The colour and gloss a paint is drawn with.
fn paint_color(paint: u8) -> [u8; 4] {
    let [r, g, b] = PALETTE[usize::from(paint).min(PALETTE.len() - 1)];
    [r, g, b, 0]
}

/// Bends the sides of a seated volume onto the planet: every corner goes
/// through the address it is, so neighbouring sides share their corners
/// exactly and a volume on a small world curves with it. `lift_m` stands
/// each side off along its own normal.
fn mesh(
    sector: Sector,
    origin: [i64; 3],
    sphere: QuadSphere,
    quads: &[Quad],
    color: impl Fn(u8) -> [u8; 4],
    lift_m: f32,
) -> VolumeMesh {
    let Some(first) = quads.first() else {
        return VolumeMesh {
            origin: DVec3::ZERO,
            vertices: Vec::new(),
            indices: Vec::new(),
        };
    };
    // The directions of every column corner the sides use, worked out once:
    // a direction costs two tangents, a height only a multiply.
    let (mut low, mut high) = ([i32::MAX; 2], [i32::MIN; 2]);
    for corner in quads.iter().flat_map(|quad| quad.corners()) {
        for i in 0..2 {
            low[i] = low[i].min(corner[i]);
            high[i] = high[i].max(corner[i]);
        }
    }
    let width = (high[0] - low[0] + 1) as usize;
    let directions: Vec<DVec3> = (low[1]..=high[1])
        .flat_map(|y| (low[0]..=high[0]).map(move |x| (x, y)))
        .map(|(x, y)| {
            let point = SurfacePoint::new(
                sector,
                (origin[0] + i64::from(x)) as f64,
                (origin[1] + i64::from(y)) as f64,
            );
            DVec3::from(sphere.blocks().direction(point))
        })
        .collect();
    let radius_m = sphere.radius_m();
    let at = |p: [i32; 3]| {
        let direction = directions[(p[1] - low[1]) as usize * width + (p[0] - low[0]) as usize];
        direction * (radius_m + (origin[2] + i64::from(p[2])) as f64 * BLOCK_M)
    };
    let base_at = at(first.corners()[0]);

    let mut vertices = Vec::with_capacity(quads.len() * 4);
    let mut indices = Vec::with_capacity(quads.len() * 6);
    for quad in quads {
        let corners = quad.corners().map(|p| (at(p) - base_at).as_vec3());
        // Each sector's frame is right handed (`u x v` is out), so a side
        // wound counter clockwise in cells is wound so in the world.
        let normal = (corners[1] - corners[0])
            .cross(corners[3] - corners[0])
            .normalize_or(Vec3::Z);
        let color = color(quad.paint);
        let base = vertices.len() as u32;
        for (corner, open) in corners.into_iter().zip(quad.open) {
            vertices.push(VolumeVertex {
                position: (corner + normal * lift_m).to_array(),
                normal: normal.to_array(),
                color,
                open: f32::from(open) / 3.0,
            });
        }
        let order = if quad.flipped() {
            [1, 2, 3, 1, 3, 0]
        } else {
            [0, 1, 2, 0, 2, 3]
        };
        indices.extend(order.map(|k| base + k));
    }
    VolumeMesh {
        origin: base_at,
        vertices,
        indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgen::Recipe;

    fn world() -> (Generator, SurfacePoint) {
        let generator = Generator::new(Recipe::new(1)).unwrap();
        let side = f64::from(generator.sphere().blocks().side());
        let sector = Sector::new(4).unwrap();
        (
            generator,
            SurfacePoint::new(sector, side * 0.41, side * 0.37),
        )
    }

    /// The middle of the top of a cell.
    fn top(build: &Build, sphere: QuadSphere, cell: [i32; 3]) -> DVec3 {
        let site = &build.sites[0];
        let low = site.corner(sphere, [cell[0], cell[1], cell[2] + 1]);
        let high = site.corner(sphere, [cell[0] + 1, cell[1] + 1, cell[2] + 1]);
        (low + high) / 2.0
    }

    /// An eye at `position` looking at `target`, the pointer in the middle.
    fn eye_at(position: DVec3, target: DVec3) -> Eye {
        let look = (target - position).normalize();
        let right = look.cross(position.normalize()).normalize();
        let back = -look;
        let up = back.cross(right);
        Eye {
            position,
            rotation: DQuat::from_mat3(&glam::DMat3::from_cols(right, up, back)),
            fov_y: 1.0,
            aspect: 1.5,
            pointer: [0.5, 0.5],
        }
    }

    /// Puts the pointer over a point of the world in front of the eye.
    fn point_at(eye: &mut Eye, p: DVec3) {
        let local = eye.rotation.inverse() * (p - eye.position);
        let tan = (eye.fov_y / 2.0).tan();
        let [x, y] = [local.x / -local.z / tan, local.y / -local.z / tan];
        eye.pointer = [(x / eye.aspect + 1.0) / 2.0, (1.0 - y) / 2.0];
    }

    /// A stroke from the top of one cell to the top of another, from high
    /// over the middle of the two, looking down.
    fn stroke(build: &mut Build, sphere: QuadSphere, from: [i32; 3], to: [i32; 3]) {
        let (a, b) = (top(build, sphere, from), top(build, sphere, to));
        let middle = (a + b) / 2.0;
        let mut eye = eye_at(middle + middle.normalize() * 20.0, middle);
        point_at(&mut eye, a);
        build.update(sphere, &eye, true, false);
        point_at(&mut eye, b);
        build.update(sphere, &eye, true, false);
        build.update(sphere, &eye, false, false);
    }

    #[test]
    fn a_volume_opens_on_flat_ground_at_the_body() {
        let (mut generator, point) = world();
        let mut build = Build::default();
        let stamp = build.open(&mut generator, point).expect("dry land");
        let direction = generator.sphere().blocks().direction(point);
        assert_eq!(generator.sample(direction).height_m, stamp.height_m());
        assert!(build.covers(point));
        assert_eq!(
            build.open(&mut generator, point).err(),
            Some(BuildRefusal::Neighbour)
        );
    }

    #[test]
    fn a_drag_on_the_floor_lays_a_row_and_a_body_stands_on_it() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        build.set_paint(4);
        // Aimed at the floor, the new cells go on it.
        stroke(&mut build, sphere, [10, 20, -1], [14, 20, -1]);
        let volume = &build.sites[0].volume;
        for x in 10..=14 {
            assert_eq!(volume.get([x, 20, 0]).paint(), Some(4), "{x}");
        }
        assert!(volume.get([15, 20, 0]).is_air());
        assert!(!build.drawn.is_empty());

        let site = &build.sites[0];
        let on = SurfacePoint::new(
            site.sector,
            site.origin[0] as f64 + 12.5,
            site.origin[1] as f64 + 20.5,
        );
        let footing = build.footing(on, site.origin[2] as f64 * BLOCK_M).unwrap();
        assert_eq!(footing.floor_m, Some((site.origin[2] + 1) as f64 * BLOCK_M));
    }

    #[test]
    fn delete_and_paint_act_on_what_is_there() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        stroke(&mut build, sphere, [5, 5, -1], [7, 5, -1]);
        build.take(Some(Tool::Paint));
        build.set_paint(9);
        stroke(&mut build, sphere, [5, 5, 0], [5, 5, 0]);
        build.take(Some(Tool::Delete));
        stroke(&mut build, sphere, [7, 5, 0], [7, 5, 0]);
        let volume = &build.sites[0].volume;
        assert_eq!(volume.get([5, 5, 0]).paint(), Some(9));
        assert_eq!(volume.get([6, 5, 0]).paint(), Some(0));
        assert!(volume.get([7, 5, 0]).is_air());
    }

    #[test]
    fn the_ghost_shows_the_stroke_and_goes_with_the_tool() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        let floor = top(&build, sphere, [3, 3, -1]);
        let eye = eye_at(floor + floor.normalize() * 5.0, floor);
        build.update(sphere, &eye, false, false);
        assert_eq!(build.ghost(), Some(GHOST));
        build.take(None);
        build.update(sphere, &eye, false, false);
        assert_eq!(build.ghost(), None);
    }

    #[test]
    fn a_click_lays_one_cell_however_low_the_eye() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        let floor = top(&build, sphere, [20, 20, -1]);
        let back =
            build.sites[0].corner(sphere, [14, 20, 0]) - build.sites[0].corner(sphere, [20, 20, 0]);
        // Low and far: a plane read half a cell up would land cells away.
        let mut eye = eye_at(floor + back + floor.normalize() * 0.8, floor);
        point_at(&mut eye, floor);
        build.update(sphere, &eye, true, false);
        build.update(sphere, &eye, false, false);
        let volume = &build.sites[0].volume;
        let laid: Vec<[i32; 3]> = volume
            .bounds()
            .cells()
            .filter(|&at| !volume.get(at).is_air())
            .collect();
        assert_eq!(laid, vec![[20, 20, 0]]);
    }

    /// A low eye eight cells back from a floor cell, the pointer on it,
    /// that presses, moves the pointer up the view and lets go.
    fn up_the_view(build: &mut Build, sphere: QuadSphere, upright: bool) -> Vec<[i32; 3]> {
        let floor = top(build, sphere, [20, 20, -1]);
        let site = &build.sites[0];
        let back = site.corner(sphere, [12, 20, 0]) - site.corner(sphere, [20, 20, 0]);
        let mut eye = eye_at(floor + back + floor.normalize() * 2.0, floor);
        point_at(&mut eye, floor);
        build.update(sphere, &eye, true, upright);
        eye.pointer[1] -= 0.2;
        build.update(sphere, &eye, true, upright);
        build.update(sphere, &eye, false, upright);
        let volume = &build.sites[0].volume;
        volume
            .bounds()
            .cells()
            .filter(|&at| !volume.get(at).is_air())
            .collect()
    }

    #[test]
    fn a_drag_up_the_view_lays_a_slab_away_from_the_eye() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        let laid = up_the_view(&mut build, sphere, false);
        assert!(laid.len() >= 3, "{laid:?}");
        assert!(laid.iter().all(|at| at[2] == 0 && at[1] == 20), "{laid:?}");
    }

    #[test]
    fn upright_the_same_drag_stands_a_wall() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        let laid = up_the_view(&mut build, sphere, true);
        assert!(laid.len() >= 3, "{laid:?}");
        assert!(laid.iter().all(|at| at[0] == 20 && at[1] == 20), "{laid:?}");
        assert_eq!(laid.iter().map(|at| at[2]).min(), Some(0));
    }

    #[test]
    fn undo_takes_back_a_stroke_and_redo_puts_it_back() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        stroke(&mut build, sphere, [5, 5, -1], [7, 5, -1]);
        assert_eq!(build.history(), (true, false));
        build.undo(sphere);
        assert!(build.sites[0].volume.get([6, 5, 0]).is_air());
        assert_eq!(build.history(), (false, true));
        build.redo(sphere);
        assert!(!build.sites[0].volume.get([6, 5, 0]).is_air());
        // A new stroke forgets what was taken back.
        build.undo(sphere);
        stroke(&mut build, sphere, [9, 9, -1], [9, 9, -1]);
        assert_eq!(build.history(), (true, false));
    }

    #[test]
    fn escape_drops_a_stroke_half_drawn() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        let floor = top(&build, sphere, [3, 3, -1]);
        let eye = eye_at(floor + floor.normalize() * 5.0, floor);
        build.update(sphere, &eye, true, false);
        assert!(build.cancel());
        build.update(sphere, &eye, false, false);
        assert!(build.sites[0].volume.get([3, 3, 0]).is_air());
        assert!(!build.cancel());
    }

    #[test]
    fn the_ghost_is_meshed_again_after_a_stroke_lands() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        build.take(Some(Tool::Create));
        stroke(&mut build, sphere, [3, 3, -1], [3, 3, -1]);
        build.take(Some(Tool::Delete));
        build.drain_changes();
        stroke(&mut build, sphere, [3, 3, 0], [3, 3, 0]);
        // Whatever the last frame shows over the hole, it is not the cube.
        let ghost = build
            .drain_changes()
            .into_iter()
            .rev()
            .find_map(|change| match change {
                VolumeChange::Add(GHOST, mesh) => Some(Some(mesh)),
                VolumeChange::Remove(GHOST) => Some(None),
                _ => None,
            });
        assert!(ghost.is_some_and(|mesh| mesh.is_none_or(|mesh| mesh.indices.is_empty())));
    }

    #[test]
    fn sides_share_their_corners_across_cells() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        let site = &build.sites[0];
        let a = site.corner(sphere, [3, 4, 2]);
        let quads = [
            Quad {
                cell: [2, 3, 1],
                face: voxel::Face::UP,
                paint: 0,
                open: [3; 4],
            },
            Quad {
                cell: [3, 4, 1],
                face: voxel::Face::UP,
                paint: 0,
                open: [3; 4],
            },
        ];
        let mesh = mesh(site.sector, site.origin, sphere, &quads, paint_color, 0.0);
        let shared: Vec<DVec3> = mesh
            .vertices
            .iter()
            .map(|v| mesh.origin + Vec3::from(v.position).as_dvec3())
            .filter(|p| p.distance(a) < 1e-3)
            .collect();
        assert_eq!(shared.len(), 2);
        // Up is out of the planet.
        let normal = Vec3::from(mesh.vertices[0].normal).as_dvec3();
        assert!(normal.dot(a.normalize()) > 0.99);
    }
}
