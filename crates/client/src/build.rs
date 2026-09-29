//! The build layer on the client: volumes seated on the planet, the tool that
//! strokes them, and the meshes that draw them.
//!
//! A volume knows no sphere (`voxel`). Here it is seated, and the address
//! cuts it (DECISIONS 77): a sector's columns are cut into plots of
//! [`PLOT_BITS`], a volume stands over one plot, and its cell `(x, y, z)` is
//! the address `(u, v, h)` of that sector. So a cube is exactly a block of
//! the world, neighbours meet with nothing between them, and a body walks on
//! them in address space the way it walks on the ground (CLAUDE.md, simulate
//! flat, render spherical). Each corner of each side is bent onto the planet
//! on its own, which is why the sides are never merged.
//!
//! Volumes live here only: nothing is sent or kept yet (DECISIONS 76).

use std::collections::BTreeSet;

use glam::{DQuat, DVec3, Vec3};
use scene::{VolumeChange, VolumeMesh, VolumeMeshId, VolumeVertex};
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint};
use voxel::{CHUNK_BITS, Cell, Gesture, Hit, Quad, Span, Volumes, crossing};
use worldgen::{Generator, Stamp};

use crate::collision::{Footing, STEP_M};
use crate::seam::{BuildRefusal, Tool};

/// Blocks along the side of a plot, as a power of two: 64 of them, 32 metres
/// at the middle of a sector. The address of a column, less these bits, is
/// the plot it is on.
const PLOT_BITS: u32 = 6;
/// Cells a volume is tall: as tall as its plot is wide.
const HEIGHT: u32 = 1 << PLOT_BITS;
/// Metres over which the ground around a volume eases back to its own.
const BLEND_M: f64 = 8.0;
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

/// The volumes of one sector, seated: a cell is the address it has.
struct Seated {
    sector: Sector,
    volumes: Volumes,
    /// A ball around each volume, in the world: what a line of sight asks
    /// before it is bent into cells.
    balls: Vec<(DVec3, f64)>,
}

impl Seated {
    /// Where a lattice point of the sector's cells is in the world.
    fn corner(&self, sphere: QuadSphere, p: [i32; 3]) -> DVec3 {
        let point = SurfacePoint::new(self.sector, f64::from(p[0]), f64::from(p[1]));
        DVec3::from(sphere.position(point, f64::from(p[2]) * BLOCK_M))
    }

    /// A point of the world in this sector's cells, when it is over it.
    fn cells_of(&self, sphere: QuadSphere, p: DVec3) -> Option<[f64; 3]> {
        let point = sphere.blocks().surface_point(p.to_array());
        (point.sector == self.sector)
            .then(|| [point.u, point.v, (p.length() - sphere.radius_m()) / BLOCK_M])
    }

    /// Opens the volume of a plot, on a floor at `floor` blocks.
    fn open(&mut self, sphere: QuadSphere, plot: [i32; 2], floor: i32) {
        if !self.volumes.open(plot, floor) {
            return;
        }
        let Some(bounds) = self.volumes.bounds(plot) else {
            return;
        };
        let size = bounds.size().map(|n| n as i32);
        let middle = self.corner(sphere, [0, 1, 2].map(|i| bounds.min[i] + size[i] / 2));
        let radius_m = self.corner(sphere, bounds.min).distance(middle) * 1.5;
        self.balls.push((middle, radius_m));
    }

    /// A line of sight from `from` along `toward` (unit), as a path in this
    /// sector's cells: the stretch of it that could reach a volume.
    fn sight(&self, sphere: QuadSphere, from: DVec3, toward: DVec3) -> Vec<[f64; 3]> {
        let stretch = self
            .balls
            .iter()
            .filter_map(|&(middle, radius_m)| {
                let along = (middle - from).dot(toward);
                let miss2 = (middle - from).length_squared() - along * along;
                let inside2 = radius_m * radius_m - miss2;
                (inside2 >= 0.0).then(|| (along - inside2.sqrt(), along + inside2.sqrt()))
            })
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)));
        let Some((near, far)) = stretch else {
            return Vec::new();
        };
        let (near, far) = (near.max(0.0), far.min(REACH_M));
        let steps = ((far - near) / SIGHT_STEP_M).ceil().max(0.0) as usize;
        (0..=steps)
            .filter_map(|i| {
                let t = (near + i as f64 * SIGHT_STEP_M).min(far);
                self.cells_of(sphere, from + toward * t)
            })
            .collect()
    }

    /// The mesh of a chunk, named by the address of its lowest corner.
    fn mesh_id(&self, chunk: [i32; 3]) -> VolumeMeshId {
        let [u, v, h] = chunk;
        let across = |n: i32| u64::from(n as u32 >> CHUNK_BITS);
        // Heights run either side of the datum: 24 bits hold them all.
        let up = u64::from((h + (1 << 23)) as u32 & 0xff_ffff);
        VolumeMeshId((self.sector.index() as u64) << 56 | across(u) << 40 | across(v) << 24 | up)
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
    /// The sectors a volume stands on, in the order the first one opened.
    sites: Vec<Seated>,
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
        self.floor_at(point.sector, plot_of(point)).is_some()
    }

    /// The floor of the volume over a plot, in blocks, where one stands.
    fn floor_at(&self, sector: Sector, plot: [i32; 2]) -> Option<i32> {
        let site = self.sites.iter().find(|site| site.sector == sector)?;
        site.volumes.floor(plot)
    }

    /// The floor a volume over `plot` takes from those around it: that of
    /// the nearest to `point` of the volumes it touches, by a side or by a
    /// corner. A platform grows by its neighbours.
    fn floor_beside(&self, sector: Sector, plot: [i32; 2], point: SurfacePoint) -> Option<i32> {
        let side = f64::from(1u32 << PLOT_BITS);
        let away = |beside: [i32; 2]| {
            let out = |at: f64, low: i32| {
                let low = f64::from(low) * side;
                (low - at).max(at - (low + side)).max(0.0)
            };
            out(point.u, beside[0]).hypot(out(point.v, beside[1]))
        };
        (-1..=1)
            .flat_map(|dv| (-1..=1).map(move |du| [plot[0] + du, plot[1] + dv]))
            .filter_map(|beside| Some((away(beside), self.floor_at(sector, beside)?)))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, floor)| floor)
    }

    /// Opens the volume of the plot a column is on, and returns the stamp
    /// that holds its ground flat: the ground under it has to be drawn again.
    /// `None` where the volume stands already. Its floor is that of the
    /// volume beside it, or the ground at the column where none is.
    pub fn open(
        &mut self,
        generator: &mut Generator,
        point: SurfacePoint,
    ) -> Result<Option<Stamp>, BuildRefusal> {
        let sphere = generator.sphere();
        let plot = plot_of(point);
        if self.floor_at(point.sector, plot).is_some() {
            return Ok(None);
        }
        // A stamp holds within its sector, so a plot on the edge of one stays
        // nature, and the corners of a sector with it.
        let inside = sphere
            .blocks()
            .coarsened(PLOT_BITS)
            .is_some_and(|plots| plot.iter().all(|&n| 0 < n && n < plots.side() as i32 - 1));
        if !inside {
            return Err(BuildRefusal::Seam);
        }
        let ground_m = generator.sample(sphere.blocks().direction(point)).height_m;
        if ground_m < 0.0 {
            return Err(BuildRefusal::Sea);
        }
        let floor = self
            .floor_beside(point.sector, plot, point)
            .unwrap_or_else(|| libm::round(ground_m / BLOCK_M) as i32);
        let low = plot.map(|n| f64::from(n << PLOT_BITS));
        let side = f64::from(1u32 << PLOT_BITS);
        let stamp = Stamp::new(
            sphere,
            point.sector,
            [low[0], low[0] + side],
            [low[1], low[1] + side],
            f64::from(floor) * BLOCK_M,
            BLEND_M,
        );
        generator.stamp(stamp);
        let site = match self
            .sites
            .iter()
            .position(|site| site.sector == point.sector)
        {
            Some(site) => site,
            None => {
                self.sites.push(Seated {
                    sector: point.sector,
                    volumes: Volumes::new(PLOT_BITS, HEIGHT),
                    balls: Vec::new(),
                });
                self.sites.len() - 1
            }
        };
        self.sites[site].open(sphere, plot, floor);
        Ok(Some(stamp))
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
                .volumes
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
                .volumes
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
            let (across, at) = stroke.across;
            // On the layer the stroke goes in, whatever is behind it, and
            // over as many volumes as it reaches: a gesture takes of each
            // what it holds.
            if let Some(p) = crossing(&paths[stroke.site], across, at) {
                let mut end = p.map(|n| n.floor() as i32);
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

    /// The nearest thing the line of sight meets, over any sector.
    fn aim_at(&self, sphere: QuadSphere, from: DVec3, paths: &[Vec<[f64; 3]>]) -> Option<Aim> {
        paths
            .iter()
            .enumerate()
            .filter_map(|(site, path)| {
                let hit = self.sites[site].volumes.trace(path)?;
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
        self.sites[aim.site].volumes.holds(cell).then_some(Stroke {
            site: aim.site,
            tool,
            start: cell,
            across: (face.axis, surface),
            end: cell,
        })
    }

    /// Lays a gesture on the first sector built on, as a stroke would.
    #[cfg(test)]
    pub(crate) fn lay(&mut self, sphere: QuadSphere, gesture: Gesture) {
        self.apply(sphere, 0, gesture);
    }

    /// The cells of the volume over a column.
    #[cfg(test)]
    pub(crate) fn cells_over(&self, point: SurfacePoint) -> Option<Span> {
        let site = self.sites.iter().find(|site| site.sector == point.sector)?;
        site.volumes.bounds(plot_of(point))
    }

    /// Applies a gesture, keeps it to take back, and draws what it changed.
    fn apply(&mut self, sphere: QuadSphere, site: usize, gesture: Gesture) {
        let volumes = &mut self.sites[site].volumes;
        // What is kept is what volumes hold of the stroke, however far it ran.
        let Some(span) = volumes.held(gesture.span()) else {
            return;
        };
        let before = volumes.cells(span);
        let Some(changed) = volumes.apply(gesture) else {
            return;
        };
        let after = volumes.cells(span);
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
        for chunk in seated.volumes.chunks_in(changed.grown(1)) {
            let id = seated.mesh_id(chunk);
            let quads = seated.volumes.faces(chunk);
            if quads.is_empty() {
                if self.drawn.remove(&id) {
                    self.changes.push(VolumeChange::Remove(id));
                }
                continue;
            }
            let mesh = mesh(seated.sector, sphere, &quads, paint_color, 0.0);
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
        let color = match gesture {
            Gesture::Create { paint, .. } | Gesture::Paint { paint, .. } => paint_color(paint),
            Gesture::Delete { .. } => {
                let [r, g, b] = DELETE_COLOR;
                [r, g, b, 0]
            }
        };
        let quads = seated.volumes.ghost(gesture);
        let mut mesh = mesh(seated.sector, sphere, &quads, |_| color, GHOST_LIFT_M);
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
    /// highest cell under any part of it, over whichever volumes it
    /// straddles, and stops short of a wall.
    pub fn footing(&self, point: SurfacePoint, feet_m: f64) -> Option<Footing> {
        let site = self.sites.iter().find(|site| site.sector == point.sector)?;
        let reach = (feet_m + STEP_M) / BLOCK_M;
        let columns = |at: f64| (at - BODY_HALF).floor() as i32..=(at + BODY_HALF).floor() as i32;
        let metres = |cells: i32| f64::from(cells) * BLOCK_M;
        columns(point.u)
            .flat_map(|x| columns(point.v).map(move |y| (x, y)))
            .filter_map(|(x, y)| site.volumes.gap(x, y, reach))
            .map(|gap| Footing {
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

/// The plot a column is on: its address, less the bits of a plot.
fn plot_of(point: SurfacePoint) -> [i32; 2] {
    [point.u, point.v].map(|at| at.floor() as i32 >> PLOT_BITS)
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

/// Bends the sides of a sector's cells onto the planet: every corner goes
/// through the address it is, so neighbouring sides share their corners
/// exactly, across two volumes as within one, and a volume on a small world
/// curves with it. `lift_m` stands each side off along its own normal.
fn mesh(
    sector: Sector,
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
            let point = SurfacePoint::new(sector, f64::from(x), f64::from(y));
            DVec3::from(sphere.blocks().direction(point))
        })
        .collect();
    let radius_m = sphere.radius_m();
    let at = |p: [i32; 3]| {
        let direction = directions[(p[1] - low[1]) as usize * width + (p[0] - low[0]) as usize];
        direction * (radius_m + f64::from(p[2]) * BLOCK_M)
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

    /// A world with a volume open where [`world`] stands, and the address of
    /// its first cell: the tests speak in cells counted from it.
    fn opened() -> (Generator, Build, [i32; 3]) {
        let (mut generator, point) = world();
        let mut build = Build::default();
        build.open(&mut generator, point).expect("dry land");
        let origin = build.cells_over(point).expect("an open volume").min;
        (generator, build, origin)
    }

    /// A column `plots` plots along `u` from another.
    fn along(point: SurfacePoint, plots: i32) -> SurfacePoint {
        let blocks = f64::from(plots << PLOT_BITS);
        SurfacePoint::new(point.sector, point.u + blocks, point.v)
    }

    fn at(origin: [i32; 3], cell: [i32; 3]) -> [i32; 3] {
        [0, 1, 2].map(|i| origin[i] + cell[i])
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

    /// Every cell that is not air in the volume the tests open, counted
    /// from its first.
    fn laid(build: &Build, origin: [i32; 3]) -> Vec<[i32; 3]> {
        let volumes = &build.sites[0].volumes;
        let plot = volumes.plot_of(origin[0], origin[1]);
        let bounds = volumes.bounds(plot).expect("an open volume");
        bounds
            .cells()
            .filter(|&cell| !volumes.get(cell).is_air())
            .map(|cell| [0, 1, 2].map(|i| cell[i] - origin[i]))
            .collect()
    }

    #[test]
    fn a_volume_opens_over_the_plot_under_the_body() {
        let (mut generator, point) = world();
        let mut build = Build::default();
        let stamp = build
            .open(&mut generator, point)
            .expect("dry land")
            .expect("no volume yet");
        let direction = generator.sphere().blocks().direction(point);
        assert_eq!(generator.sample(direction).height_m, stamp.height_m());
        assert!(build.covers(point));
        // The address cuts it: wherever on the plot the body stood.
        let cells = build.cells_over(point).unwrap();
        assert_eq!(cells.size(), [64, 64, 64]);
        assert_eq!(cells.min[0] % 64, 0);
        assert_eq!(cells.min[1] % 64, 0);
        assert!(cells.contains([point.u as i32, point.v as i32, cells.min[2]]));
        assert_eq!(f64::from(cells.min[2]) * BLOCK_M, stamp.height_m());
        // Where one stands, nothing opens and the ground stays as it is.
        assert_eq!(build.open(&mut generator, point), Ok(None));
        assert_eq!(generator.stamps().len(), 1);
    }

    #[test]
    fn a_plot_on_the_edge_of_a_sector_stays_nature() {
        let (mut generator, point) = world();
        let side = f64::from(generator.sphere().blocks().side());
        let mut build = Build::default();
        for (u, v) in [(10.0, point.v), (point.u, side - 10.0), (side - 1.0, 1.0)] {
            let edge = SurfacePoint::new(point.sector, u, v);
            let opened = build.open(&mut generator, edge);
            assert_eq!(opened.err(), Some(BuildRefusal::Seam), "{u} {v}");
        }
        // One plot in, the edge is no reason: the sea may be.
        let inside = SurfacePoint::new(point.sector, 70.0, side - 70.0);
        let opened = build.open(&mut generator, inside);
        assert_ne!(opened.err(), Some(BuildRefusal::Seam));
    }

    #[test]
    fn a_volume_takes_the_floor_of_the_one_beside_it() {
        let (mut generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.open(&mut generator, point).unwrap();
        let floor = build.cells_over(point).unwrap().min[2];
        // Two plots away nothing touches it: the ground there is its own.
        let apart = along(point, 2);
        let ground_m = generator.sample(sphere.blocks().direction(apart)).height_m;
        assert_ne!(libm::round(ground_m / BLOCK_M) as i32, floor);
        let beside = along(point, 1);
        let stamp = build.open(&mut generator, beside).unwrap().unwrap();
        assert_eq!(stamp.height_m(), f64::from(floor) * BLOCK_M);
        assert_eq!(build.cells_over(beside).unwrap().min[2], floor);
        // And the platform goes on: the third joins the second.
        build.open(&mut generator, apart).unwrap();
        assert_eq!(build.cells_over(apart).unwrap().min[2], floor);
        // The ground is one floor under the three of them.
        for plots in 0..3 {
            let on = sphere.blocks().direction(along(point, plots));
            assert_eq!(generator.sample(on).height_m, stamp.height_m());
        }
    }

    #[test]
    fn a_stroke_stands_over_two_neighbours() {
        let (mut generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        let (_, point) = world();
        build.open(&mut generator, along(point, 1)).unwrap();
        build.take(Some(Tool::Create));
        build.set_paint(4);
        stroke(
            &mut build,
            sphere,
            at(origin, [60, 20, -1]),
            at(origin, [67, 20, -1]),
        );
        let volumes = &build.sites[0].volumes;
        for x in 60..=67 {
            let cell = at(origin, [x, 20, 0]);
            assert_eq!(volumes.get(cell).paint(), Some(4), "{x}");
        }
        // A chunk of each volume is drawn, and one stroke takes both back.
        assert_eq!(build.drawn.len(), 2);
        build.undo(sphere);
        let volumes = &build.sites[0].volumes;
        for x in 60..=67 {
            assert!(volumes.get(at(origin, [x, 20, 0])).is_air(), "{x}");
        }
        assert!(build.drawn.is_empty());
    }

    #[test]
    fn a_stroke_stops_where_no_volume_stands() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            sphere,
            at(origin, [60, 20, -1]),
            at(origin, [67, 20, -1]),
        );
        let row: Vec<[i32; 3]> = (60..64).map(|x| [x, 20, 0]).collect();
        assert_eq!(laid(&build, origin), row);
    }

    #[test]
    fn a_body_stands_astride_two_volumes() {
        let (mut generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        let (_, point) = world();
        build.open(&mut generator, along(point, 1)).unwrap();
        // One cube at the edge of the second volume, and a body half on it,
        // its middle over the first.
        let cube = at(origin, [64, 20, 0]);
        build.lay(
            sphere,
            Gesture::Create {
                span: Span::cell(cube),
                paint: 0,
            },
        );
        let floor_m = f64::from(origin[2]) * BLOCK_M;
        let astride = SurfacePoint::new(
            point.sector,
            f64::from(cube[0]) - 0.3,
            f64::from(cube[1]) + 0.5,
        );
        let footing = build.footing(astride, floor_m).unwrap();
        assert_eq!(footing.floor_m, Some(floor_m + BLOCK_M));
        // Off the platform the volumes hold nothing up.
        let off = SurfacePoint::new(point.sector, f64::from(origin[0]) - 5.0, point.v);
        assert_eq!(build.footing(off, floor_m), None);
    }

    #[test]
    fn a_drag_on_the_floor_lays_a_row_and_a_body_stands_on_it() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        build.set_paint(4);
        // Aimed at the floor, the new cells go on it.
        stroke(
            &mut build,
            sphere,
            at(origin, [10, 20, -1]),
            at(origin, [14, 20, -1]),
        );
        let volumes = &build.sites[0].volumes;
        for x in 10..=14 {
            let cell = at(origin, [x, 20, 0]);
            assert_eq!(volumes.get(cell).paint(), Some(4), "{x}");
        }
        assert!(volumes.get(at(origin, [15, 20, 0])).is_air());
        assert!(!build.drawn.is_empty());

        let on = SurfacePoint::new(
            build.sites[0].sector,
            f64::from(origin[0]) + 12.5,
            f64::from(origin[1]) + 20.5,
        );
        let footing = build.footing(on, f64::from(origin[2]) * BLOCK_M).unwrap();
        assert_eq!(footing.floor_m, Some(f64::from(origin[2] + 1) * BLOCK_M));
    }

    #[test]
    fn delete_and_paint_act_on_what_is_there() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            sphere,
            at(origin, [5, 5, -1]),
            at(origin, [7, 5, -1]),
        );
        build.take(Some(Tool::Paint));
        build.set_paint(9);
        stroke(
            &mut build,
            sphere,
            at(origin, [5, 5, 0]),
            at(origin, [5, 5, 0]),
        );
        build.take(Some(Tool::Delete));
        stroke(
            &mut build,
            sphere,
            at(origin, [7, 5, 0]),
            at(origin, [7, 5, 0]),
        );
        let volumes = &build.sites[0].volumes;
        assert_eq!(volumes.get(at(origin, [5, 5, 0])).paint(), Some(9));
        assert_eq!(volumes.get(at(origin, [6, 5, 0])).paint(), Some(0));
        assert!(volumes.get(at(origin, [7, 5, 0])).is_air());
    }

    #[test]
    fn the_ghost_shows_the_stroke_and_goes_with_the_tool() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        let floor = top(&build, sphere, at(origin, [3, 3, -1]));
        let eye = eye_at(floor + floor.normalize() * 5.0, floor);
        build.update(sphere, &eye, false, false);
        assert_eq!(build.ghost(), Some(GHOST));
        build.take(None);
        build.update(sphere, &eye, false, false);
        assert_eq!(build.ghost(), None);
    }

    #[test]
    fn a_click_lays_one_cell_however_low_the_eye() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        let floor = top(&build, sphere, at(origin, [20, 20, -1]));
        let site = &build.sites[0];
        let back = site.corner(sphere, at(origin, [14, 20, 0]))
            - site.corner(sphere, at(origin, [20, 20, 0]));
        // Low and far: a plane read half a cell up would land cells away.
        let mut eye = eye_at(floor + back + floor.normalize() * 0.8, floor);
        point_at(&mut eye, floor);
        build.update(sphere, &eye, true, false);
        build.update(sphere, &eye, false, false);
        assert_eq!(laid(&build, origin), vec![[20, 20, 0]]);
    }

    /// A low eye eight cells back from a floor cell, the pointer on it,
    /// that presses, moves the pointer up the view and lets go.
    fn up_the_view(
        build: &mut Build,
        sphere: QuadSphere,
        origin: [i32; 3],
        upright: bool,
    ) -> Vec<[i32; 3]> {
        let floor = top(build, sphere, at(origin, [20, 20, -1]));
        let site = &build.sites[0];
        let back = site.corner(sphere, at(origin, [12, 20, 0]))
            - site.corner(sphere, at(origin, [20, 20, 0]));
        let mut eye = eye_at(floor + back + floor.normalize() * 2.0, floor);
        point_at(&mut eye, floor);
        build.update(sphere, &eye, true, upright);
        eye.pointer[1] -= 0.2;
        build.update(sphere, &eye, true, upright);
        build.update(sphere, &eye, false, upright);
        laid(build, origin)
    }

    #[test]
    fn a_drag_up_the_view_lays_a_slab_away_from_the_eye() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        let laid = up_the_view(&mut build, generator.sphere(), origin, false);
        assert!(laid.len() >= 3, "{laid:?}");
        assert!(laid.iter().all(|at| at[2] == 0 && at[1] == 20), "{laid:?}");
    }

    #[test]
    fn upright_the_same_drag_stands_a_wall() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        let laid = up_the_view(&mut build, generator.sphere(), origin, true);
        assert!(laid.len() >= 3, "{laid:?}");
        assert!(laid.iter().all(|at| at[0] == 20 && at[1] == 20), "{laid:?}");
        assert_eq!(laid.iter().map(|at| at[2]).min(), Some(0));
    }

    #[test]
    fn undo_takes_back_a_stroke_and_redo_puts_it_back() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            sphere,
            at(origin, [5, 5, -1]),
            at(origin, [7, 5, -1]),
        );
        let middle = at(origin, [6, 5, 0]);
        assert_eq!(build.history(), (true, false));
        build.undo(sphere);
        assert!(build.sites[0].volumes.get(middle).is_air());
        assert_eq!(build.history(), (false, true));
        build.redo(sphere);
        assert!(!build.sites[0].volumes.get(middle).is_air());
        // A new stroke forgets what was taken back.
        build.undo(sphere);
        stroke(
            &mut build,
            sphere,
            at(origin, [9, 9, -1]),
            at(origin, [9, 9, -1]),
        );
        assert_eq!(build.history(), (true, false));
    }

    #[test]
    fn escape_drops_a_stroke_half_drawn() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        let floor = top(&build, sphere, at(origin, [3, 3, -1]));
        let eye = eye_at(floor + floor.normalize() * 5.0, floor);
        build.update(sphere, &eye, true, false);
        assert!(build.cancel());
        build.update(sphere, &eye, false, false);
        assert!(build.sites[0].volumes.get(at(origin, [3, 3, 0])).is_air());
        assert!(!build.cancel());
    }

    #[test]
    fn the_ghost_is_meshed_again_after_a_stroke_lands() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            sphere,
            at(origin, [3, 3, -1]),
            at(origin, [3, 3, -1]),
        );
        build.take(Some(Tool::Delete));
        build.drain_changes();
        stroke(
            &mut build,
            sphere,
            at(origin, [3, 3, 0]),
            at(origin, [3, 3, 0]),
        );
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
    fn a_chunk_has_a_mesh_of_its_own_in_every_volume_and_sector() {
        let seated = |sector: u8| Seated {
            sector: Sector::new(sector).unwrap(),
            volumes: Volumes::new(PLOT_BITS, HEIGHT),
            balls: Vec::new(),
        };
        let chunks = [[64, 128, 40], [80, 128, 40], [64, 144, 40], [64, 128, -40]];
        let mut ids = BTreeSet::new();
        for sector in [0, 5] {
            for chunk in chunks {
                ids.insert(seated(sector).mesh_id(chunk));
            }
        }
        assert_eq!(ids.len(), 8);
        assert!(!ids.contains(&GHOST));
    }

    #[test]
    fn sides_share_their_corners_across_cells() {
        let (generator, build, origin) = opened();
        let sphere = generator.sphere();
        let site = &build.sites[0];
        let a = site.corner(sphere, at(origin, [3, 4, 2]));
        let quads = [
            Quad {
                cell: at(origin, [2, 3, 1]),
                face: voxel::Face::UP,
                paint: 0,
                open: [3; 4],
            },
            Quad {
                cell: at(origin, [3, 4, 1]),
                face: voxel::Face::UP,
                paint: 0,
                open: [3; 4],
            },
        ];
        let mesh = mesh(site.sector, sphere, &quads, paint_color, 0.0);
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
