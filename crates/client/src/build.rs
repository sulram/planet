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
//! The ground under a volume stays as nature made it (DECISIONS 78). What a
//! build stands on is a platform, built where the body stands: a slab as
//! high as the highest ground under it, on pillars down to the ground.
//!
//! Volumes live here only: nothing is sent or kept yet (DECISIONS 76).

use std::collections::BTreeSet;

use glam::{DQuat, DVec3, Vec3};
use scene::{VolumeChange, VolumeMesh, VolumeMeshId, VolumeVertex};
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint};
use voxel::{CHUNK, CHUNK_BITS, Cell, Gesture, Hit, Platform, Quad, Span, Volumes, crossing};
use worldgen::Generator;

use crate::collision::{Footing, STEP_M};
use crate::grass::TUFT_M;
use crate::seam::{Base, BuildRefusal, Tool};

/// Blocks along the side of a plot, as a power of two: 64 of them, 32 metres
/// at the middle of a sector. The address of a column, less these bits, is
/// the plot it is on.
const PLOT_BITS: u32 = 6;
/// Cells a volume rises over the highest ground of its plot: as many as the
/// plot is wide.
const HEIGHT: i32 = 1 << PLOT_BITS;
/// Blocks between the columns whose ground is read to find how low and how
/// high the ground of a plot stands.
const SURVEY: i32 = 8;
/// Cells a volume holds under the lowest ground its survey found: the dip
/// between two columns of it.
const UNDER: i32 = 8;
/// The sides a platform can have, in cells: powers of two, up to a plot.
pub const PLATFORMS: [u32; 4] = [8, 16, 32, 64];
/// Metres between the points of a line of sight where the ground is asked
/// whether it hides what the pointer is over.
const GROUND_STEP_M: f64 = 1.0;
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
/// Rows of the ground under a platform read in one update: a platform of 64
/// reads 65 rows of 65 corners, each a sample of the ground in full detail.
const GROUND_ROWS_PER_UPDATE: usize = 13;
/// Chunks meshed in one update, the nearest the eye first. A change to more
/// is drawn over as many updates as it takes, each chunk keeping the mesh it
/// had until its new one is made, so no frame pays for a platform whole.
const MESHES_PER_UPDATE: usize = 8;

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

    /// Opens the volume of a plot, holding the blocks from `low` to `height`
    /// over it.
    fn open(&mut self, sphere: QuadSphere, plot: [i32; 2], low: i32, height: u32) {
        if !self.volumes.open(plot, low, height) {
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
/// through it, to the cell the pointer is over, or to where it meets that
/// layer when it is over nothing of it.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Stroke {
    site: usize,
    tool: Tool,
    start: [i32; 3],
    /// The side the stroke started on: the axis it faces, and where along it
    /// the surface is.
    side: (usize, f64),
    /// The axes of the three layers through the start, in the order a hand
    /// turns the stroke through them: the side it started on first.
    turns: [usize; 3],
    /// The axis the layer is across, and where along it the pointer is read.
    /// A stroke lies on the side it started on and is read on that side;
    /// turned, or led onto another surface, it lies across another axis and
    /// is read on its face toward the eye. Read anywhere else, the cell
    /// under the pointer is not the one the stroke shows.
    across: (usize, f64),
    end: [i32; 3],
}

impl Stroke {
    fn span(self) -> Span {
        Span::between(self.start, self.end)
    }

    /// The cell a tool takes where it aims: a new one goes in the air before
    /// the side hit, and what is taken away or repainted is the cell hit.
    fn cell(tool: Tool, hit: Hit) -> [i32; 3] {
        match tool {
            Tool::Create => hit.before(),
            Tool::Delete | Tool::Paint => hit.cell,
        }
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

/// A platform asked for, and the ground under it read so far: the height of
/// each corner of each column, in blocks, row by row.
struct Laying {
    site: usize,
    square: Platform,
    base: Base,
    paint: u8,
    corners: Vec<f64>,
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
    /// The side of the next platform, in cells, as a power of two.
    platform_bits: u32,
    aim: Option<Aim>,
    stroke: Option<Stroke>,
    using: bool,
    /// Whether the key that turns a stroke was down last frame: a stroke
    /// turns as it goes down.
    turning: bool,
    /// How many times the stroke being drawn, or the next one, was turned.
    turned: usize,
    /// What the ghost shows now: it is meshed again only when that changes.
    ghost: Option<Gesture>,
    drawn: BTreeSet<VolumeMeshId>,
    /// Chunks owed a mesh since their cells changed, by site.
    stale: BTreeSet<(usize, [i32; 3])>,
    /// The platform asked for whose ground is still being read.
    laying: Option<Laying>,
    /// The height of the top of the platform laid last, metres, until the
    /// client takes it.
    laid: Option<f64>,
    changes: Vec<VolumeChange>,
    /// Where cells changed near enough the ground to cover or bare a tuft,
    /// as a cap of the body: its middle, unit, and its angle.
    touched: Vec<(DVec3, f64)>,
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
            platform_bits: 4,
            aim: None,
            stroke: None,
            using: false,
            turning: false,
            turned: 0,
            ghost: None,
            drawn: BTreeSet::new(),
            stale: BTreeSet::new(),
            laying: None,
            laid: None,
            changes: Vec::new(),
            touched: Vec::new(),
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
        self.turned = 0;
        self.aim = None;
    }

    pub fn set_paint(&mut self, paint: u8) {
        self.paint = paint.min(PALETTE.len() as u8 - 1);
    }

    /// The side of the platform that is laid next, in cells.
    pub fn platform(&self) -> u32 {
        1 << self.platform_bits
    }

    /// Picks the side of the platform that is laid next: the one on offer
    /// nearest to `side` cells.
    pub fn set_platform(&mut self, side: u32) {
        let nearest = PLATFORMS.into_iter().min_by_key(|on| on.abs_diff(side));
        self.platform_bits = nearest.unwrap_or(PLATFORMS[0]).trailing_zeros();
    }

    /// Whether a volume stands on a column.
    pub fn covers(&self, point: SurfacePoint) -> bool {
        self.sites
            .iter()
            .find(|site| site.sector == point.sector)
            .is_some_and(|site| site.volumes.is_open(plot_of(point)))
    }

    /// Opens the volume of the plot a column is on, where none stands. The
    /// ground stays as it is, and the volume holds the blocks from the lowest
    /// of it to [`HEIGHT`] over the highest.
    fn open(&mut self, generator: &Generator, point: SurfacePoint) -> Result<(), BuildRefusal> {
        let sphere = generator.sphere();
        let plot = plot_of(point);
        // The corners of a sector are nature, and a build does not fold over
        // a seam: a plot on the edge of its sector stays as it is.
        let inside = sphere
            .blocks()
            .coarsened(PLOT_BITS)
            .is_some_and(|plots| plot.iter().all(|&n| 0 < n && n < plots.side() as i32 - 1));
        if !inside {
            return Err(BuildRefusal::Seam);
        }
        if ground(generator, point.sector, point.u, point.v) < 0.0 {
            return Err(BuildRefusal::Sea);
        }
        if self.covers(point) {
            return Ok(());
        }
        let low = plot.map(|n| n << PLOT_BITS);
        let side = 1 << PLOT_BITS;
        let heights = (0..=side).step_by(SURVEY as usize).flat_map(|dv| {
            (0..=side).step_by(SURVEY as usize).map(move |du| {
                let (u, v) = (f64::from(low[0] + du), f64::from(low[1] + dv));
                ground(generator, point.sector, u, v)
            })
        });
        let (lowest, highest) = heights.fold((f64::MAX, f64::MIN), |(lo, hi), blocks| {
            (lo.min(blocks), hi.max(blocks))
        });
        let bottom = lowest.floor() as i32 - UNDER;
        let top = highest.ceil() as i32 + HEIGHT;
        let site = match self
            .sites
            .iter()
            .position(|site| site.sector == point.sector)
        {
            Some(site) => site,
            None => {
                self.sites.push(Seated {
                    sector: point.sector,
                    volumes: Volumes::new(PLOT_BITS),
                    balls: Vec::new(),
                });
                self.sites.len() - 1
            }
        };
        self.sites[site].open(sphere, plot, bottom, (top - bottom) as u32);
        Ok(())
    }

    /// Asks for a platform where a body stands, opening the volume of its
    /// plot if none stands there: a slab of the side picked, in the paint in
    /// hand, its top over the highest ground under it, on `base` down to the
    /// ground. The ground under it is read a few rows an update and the
    /// platform lands when all of it is read, a stroke like any other, kept
    /// to take back; [`Build::take_laid`] says how high its top stands. A
    /// platform asked for while another is being read takes its place.
    pub fn lay_platform(
        &mut self,
        generator: &Generator,
        point: SurfacePoint,
        base: Base,
    ) -> Result<(), BuildRefusal> {
        self.open(generator, point)?;
        let (x, y) = (point.u.floor() as i32, point.v.floor() as i32);
        let square = Platform::over(x, y, self.platform_bits, 0);
        let across = square.side as usize + 1;
        let site = self
            .sites
            .iter()
            .position(|site| site.sector == point.sector)
            .expect("a volume was opened on this sector");
        self.laying = Some(Laying {
            site,
            square,
            base,
            paint: self.paint,
            corners: Vec::with_capacity(across * across),
        });
        Ok(())
    }

    /// Reads up to `rows` more rows of the ground under the platform being
    /// laid, and lays it once every corner of every column is read. True
    /// when it landed.
    fn read_ground(&mut self, generator: &Generator, rows: usize) -> bool {
        let Some(laying) = &mut self.laying else {
            return false;
        };
        let sector = self.sites[laying.site].sector;
        let [x0, y0] = laying.square.corner;
        let across = laying.square.side as usize + 1;
        let read = laying.corners.len() / across;
        for row in read..(read.saturating_add(rows)).min(across) {
            laying.corners.extend((0..across).map(|dx| {
                let (u, v) = (x0 + dx as i32, y0 + row as i32);
                ground(generator, sector, f64::from(u), f64::from(v))
            }));
        }
        if laying.corners.len() < across * across {
            return false;
        }
        let Laying {
            site,
            square,
            base,
            paint,
            corners,
        } = self.laying.take().expect("a platform being laid");
        let highest = corners.iter().copied().fold(f64::MIN, f64::max);
        let platform = Platform {
            top: highest.ceil() as i32,
            ..square
        };
        // A base reaches the lowest ground at the foot of its column, so
        // no ground shows under it.
        let under = |x: i32, y: i32| {
            let (dx, dy) = ((x - x0) as usize, (y - y0) as usize);
            let at = |dx: usize, dy: usize| corners[dy * across + dx];
            let lowest = at(dx, dy)
                .min(at(dx + 1, dy))
                .min(at(dx, dy + 1))
                .min(at(dx + 1, dy + 1));
            lowest.floor() as i32
        };
        let lowest = corners.iter().copied().fold(f64::MAX, f64::min);
        let mut reach = platform.slab();
        reach.min[2] = reach.min[2].min(lowest.floor() as i32);
        let base = match base {
            Base::Pillars => voxel::Base::Pillars,
            Base::Solid => voxel::Base::Solid,
        };
        self.change(generator, site, reach, |volumes| {
            platform
                .gestures(base, under, paint)
                .into_iter()
                .filter_map(|gesture| volumes.apply(gesture))
                .reduce(|a, b| a.with(b.min).with(b.max))
        });
        self.laid = Some(f64::from(platform.top) * BLOCK_M);
        true
    }

    /// The height of the top of the platform laid since the last call,
    /// metres: what a body standing lower is lifted onto.
    pub fn take_laid(&mut self) -> Option<f64> {
        self.laid.take()
    }

    /// Asks for a platform and lays it at once, however long the ground
    /// takes to read: for tests.
    #[cfg(test)]
    pub(crate) fn lay_platform_now(
        &mut self,
        generator: &Generator,
        point: SurfacePoint,
        base: Base,
    ) -> Result<f64, BuildRefusal> {
        self.lay_platform(generator, point, base)?;
        self.read_ground(generator, usize::MAX);
        Ok(self.take_laid().expect("a platform laid"))
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
        self.stale.clear();
        self.laying = None;
        self.laid = None;
        self.touched.clear();
        self.aim = None;
        self.stroke = None;
        self.done.clear();
        self.undone.clear();
    }

    /// Drops the stroke being drawn, if there is one. True when there was.
    pub fn cancel(&mut self) -> bool {
        self.turned = 0;
        self.stroke.take().is_some()
    }

    /// Whether there is a stroke to take back, and one to put back.
    pub fn history(&self) -> (bool, bool) {
        (!self.done.is_empty(), !self.undone.is_empty())
    }

    /// Takes back the last stroke that landed.
    pub fn undo(&mut self, generator: &Generator) {
        if let Some(change) = self.done.pop() {
            self.stroke = None;
            let changed = self.sites[change.site]
                .volumes
                .restore(change.span, &change.before);
            if let Some(changed) = changed {
                self.redraw(generator, change.site, changed);
            }
            self.undone.push(change);
        }
    }

    /// Puts back the last stroke taken back.
    pub fn redo(&mut self, generator: &Generator) {
        if let Some(change) = self.undone.pop() {
            self.stroke = None;
            let changed = self.sites[change.site]
                .volumes
                .restore(change.span, &change.after);
            if let Some(changed) = changed {
                self.redraw(generator, change.site, changed);
            }
            self.done.push(change);
        }
    }

    /// One frame of the tool: what the line of sight through the pointer
    /// meets, the stroke while the button is held, and the gesture when it
    /// is let go. Each time `turn` goes down the stroke turns to the next of
    /// the three layers through its start.
    pub fn update(&mut self, generator: &Generator, eye: &Eye, using: bool, turn: bool) {
        self.handle(generator, eye, using, turn);
        // A platform lands in an update of its own: its cells are the work
        // of that one, and their meshes start with the next.
        if !self.read_ground(generator, GROUND_ROWS_PER_UPDATE) {
            self.mesh_owed(generator.sphere(), eye.position, MESHES_PER_UPDATE);
        }
    }

    /// Whether every platform asked for is laid, and every chunk that
    /// changed is drawn as it is now.
    pub fn settled(&self) -> bool {
        self.laying.is_none() && self.stale.is_empty()
    }

    /// Lays the platform asked for and meshes every chunk owed one: for a
    /// picture that must show all of it.
    pub fn settle(&mut self, generator: &Generator, eye: DVec3) {
        self.read_ground(generator, usize::MAX);
        self.mesh_owed(generator.sphere(), eye, usize::MAX);
    }

    /// Aims, strokes and lands what the tool in hand does this update.
    fn handle(&mut self, generator: &Generator, eye: &Eye, using: bool, turn: bool) {
        let sphere = generator.sphere();
        let turned = turn && !self.turning;
        self.turning = turn;
        let Some(tool) = self.tool else {
            self.using = using;
            self.show_ghost(sphere, None);
            return;
        };
        if turned {
            self.turned = (self.turned + 1) % 3;
        }
        let (from, toward) = eye.sight();
        let paths: Vec<Vec<[f64; 3]>> = self
            .sites
            .iter()
            .map(|site| site.sight(sphere, from, toward))
            .collect();
        self.aim = self.aim_at(generator, from, toward, &paths);

        let pressed = using && !self.using;
        let released = !using && self.using;
        self.using = using;
        if pressed {
            self.stroke = self.aim.and_then(|aim| {
                let mut stroke = self.start(tool, aim)?;
                let site = &self.sites[stroke.site];
                stroke.turns = turns(site, sphere, eye, stroke.start, stroke.side.0);
                Some(stroke)
            });
        }
        if let Some(mut stroke) = self.stroke {
            let site = &self.sites[stroke.site];
            let toward_eye = |axis: usize| near(site, sphere, eye, stroke.start, axis);
            if pressed || turned {
                let axis = stroke.turns[self.turned];
                stroke.across = match axis == stroke.side.0 {
                    true => stroke.side,
                    false => (axis, toward_eye(axis)),
                };
            }
            // The cell the pointer is over, read as the start was. It ends
            // the stroke when it lies in its layer. A stroke no hand turned
            // also follows the pointer onto another surface its start lies
            // on, as from the foot of a wall up the wall.
            let (axis, at) = stroke.across;
            let over = self.aim.filter(|aim| aim.site == stroke.site);
            let led = over.and_then(|aim| {
                let cell = Stroke::cell(tool, aim.hit);
                let on = aim.hit.face.axis;
                if cell[axis] == stroke.start[axis] {
                    Some((stroke.across, cell))
                } else if self.turned == 0 && cell[on] == stroke.start[on] {
                    Some(((on, toward_eye(on)), cell))
                } else {
                    None
                }
            });
            match led {
                Some((across, cell)) => {
                    stroke.across = across;
                    stroke.end = cell;
                }
                // Over nothing of the layer, the pointer is read where its
                // line of sight crosses it, whatever is behind, and over as
                // many volumes as it reaches.
                None => {
                    if let Some(p) = crossing(&paths[stroke.site], axis, at) {
                        stroke.end = p.map(|n| n.floor() as i32);
                    }
                }
            }
            // Whatever it was before it turned, the stroke is one layer
            // thick: an end kept from another layer is brought into this one.
            let (axis, _) = stroke.across;
            stroke.end[axis] = stroke.start[axis];
            self.stroke = Some(stroke);
        }
        if released {
            self.turned = 0;
            if let Some(stroke) = self.stroke.take() {
                self.apply(generator, stroke.site, stroke.gesture(self.paint));
            }
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

    /// The nearest thing the line of sight meets, over any sector, unless
    /// the ground stands between it and the eye: a cell a hill hides is not
    /// under the pointer.
    fn aim_at(
        &self,
        generator: &Generator,
        from: DVec3,
        toward: DVec3,
        paths: &[Vec<[f64; 3]>],
    ) -> Option<Aim> {
        let sphere = generator.sphere();
        let (away_m, aim) = paths
            .iter()
            .enumerate()
            .filter_map(|(site, path)| {
                let hit = self.sites[site].volumes.trace(path)?;
                let middle = self.sites[site].corner(sphere, hit.cell);
                Some(((middle - from).dot(toward), Aim { site, hit }))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))?;
        // Up to the near side of the cell, which may itself stand in the
        // ground.
        let clear_m = away_m - BLOCK_M;
        let steps = (clear_m / GROUND_STEP_M).floor().max(0.0) as usize;
        let hidden = (1..=steps).any(|i| {
            let at = from + toward * (i as f64 * GROUND_STEP_M);
            let direction = at.normalize_or(DVec3::Y).to_array();
            at.length() - sphere.radius_m() < generator.sample_at(direction, 0.0).height_m
        });
        (!hidden).then_some(aim)
    }

    /// The stroke a tool starts where it aims, if it can start there, lying
    /// on the side it aims at.
    fn start(&self, tool: Tool, aim: Aim) -> Option<Stroke> {
        let cell = Stroke::cell(tool, aim.hit);
        let face = aim.hit.face;
        let surface = f64::from(aim.hit.cell[face.axis] + i32::from(face.positive));
        self.sites[aim.site].volumes.holds(cell).then_some(Stroke {
            site: aim.site,
            tool,
            start: cell,
            side: (face.axis, surface),
            turns: [0, 1, 2].map(|turn| (face.axis + turn) % 3),
            across: (face.axis, surface),
            end: cell,
        })
    }

    /// Lays a gesture on the first sector built on, as a stroke would.
    #[cfg(test)]
    pub(crate) fn lay(&mut self, generator: &Generator, gesture: Gesture) {
        self.apply(generator, 0, gesture);
    }

    /// The cells of the volume over a column.
    #[cfg(test)]
    pub(crate) fn cells_over(&self, point: SurfacePoint) -> Option<Span> {
        let site = self.sites.iter().find(|site| site.sector == point.sector)?;
        site.volumes.bounds(plot_of(point))
    }

    /// Applies a gesture, keeps it to take back, and draws what it changed.
    fn apply(&mut self, generator: &Generator, site: usize, gesture: Gesture) {
        self.change(generator, site, gesture.span(), |volumes| {
            volumes.apply(gesture)
        });
    }

    /// Changes the cells of a box by `stroke`, which says what it changed,
    /// keeps the change to take back, and draws it.
    fn change(
        &mut self,
        generator: &Generator,
        site: usize,
        reach: Span,
        stroke: impl FnOnce(&mut Volumes) -> Option<Span>,
    ) {
        let volumes = &mut self.sites[site].volumes;
        // What is kept is what volumes hold of the stroke, however far it ran.
        let Some(span) = volumes.held(reach) else {
            return;
        };
        let before = volumes.cells(span);
        let Some(changed) = stroke(volumes) else {
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
        self.redraw(generator, site, changed);
    }

    /// Owes a mesh to every chunk whose sides a change to some cells could
    /// have changed: the cells, and one more all round, which is as far as a
    /// side or a corner looks. Where the change comes within a tuft of the
    /// ground the grass under it is to grow again.
    fn redraw(&mut self, generator: &Generator, site: usize, changed: Span) {
        let sphere = generator.sphere();
        // The ghost showed what the cells were: it is meshed again from what
        // they are now.
        if self.ghost.take().is_some() {
            self.changes.push(VolumeChange::Remove(GHOST));
        }
        let seated = &self.sites[site];
        let (low, high) = (changed.min, changed.max.map(|n| n + 1));
        let middle = [0, 1].map(|i| f64::from(low[i] + high[i]) / 2.0);
        let ends = [
            [low[0], low[1]],
            [high[0], low[1]],
            [low[0], high[1]],
            high[..2].try_into().expect("two of three"),
        ];
        let highest = ends
            .iter()
            .map(|&[x, y]| [f64::from(x), f64::from(y)])
            .chain([middle])
            .map(|[u, v]| ground(generator, seated.sector, u, v))
            .fold(f64::MIN, f64::max);
        if f64::from(low[2]) <= highest + TUFT_M / BLOCK_M {
            let centre = seated.corner(sphere, [middle[0] as i32, middle[1] as i32, low[2]]);
            let reach_m = seated.corner(sphere, low).distance(centre) + BLOCK_M;
            self.touched
                .push((centre.normalize(), reach_m / centre.length()));
        }
        for chunk in seated.volumes.chunks_in(changed.grown(1)) {
            self.stale.insert((site, chunk));
        }
    }

    /// Meshes up to `budget` of the chunks owed one, the nearest to `eye`
    /// first.
    fn mesh_owed(&mut self, sphere: QuadSphere, eye: DVec3, budget: usize) {
        if self.stale.is_empty() {
            return;
        }
        let half = CHUNK as i32 / 2;
        let mut owed: Vec<(f64, (usize, [i32; 3]))> = self
            .stale
            .iter()
            .map(|&(site, chunk)| {
                let middle = self.sites[site].corner(sphere, chunk.map(|n| n + half));
                (middle.distance_squared(eye), (site, chunk))
            })
            .collect();
        owed.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, owed) in owed.into_iter().take(budget) {
            self.stale.remove(&owed);
            let (site, chunk) = owed;
            let seated = &self.sites[site];
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
                floor_m: gap.floor.map(metres),
                ceiling_m: gap.ceiling.map(metres),
            })
            .reduce(Footing::with)
    }

    /// Whether a cell stands in the way of something rooted at a column of a
    /// sector, `u` and `v` in blocks, between two heights in metres.
    pub fn covered(&self, sector: Sector, column: [f64; 2], heights: [f64; 2]) -> bool {
        let Some(site) = self.sites.iter().find(|site| site.sector == sector) else {
            return false;
        };
        let [x, y] = column.map(|at| at.floor() as i32);
        let [from, to] = heights.map(|m| m / BLOCK_M);
        site.volumes.covers(x, y, from, to)
    }

    /// Mesh uploads and removals since the last call, in order.
    pub fn drain_changes(&mut self) -> Vec<VolumeChange> {
        core::mem::take(&mut self.changes)
    }

    /// Where the grass is to grow again since the last call: caps of the
    /// body, each a middle, unit, and an angle.
    pub fn drain_touched(&mut self) -> Vec<(DVec3, f64)> {
        core::mem::take(&mut self.touched)
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

/// How high the ground stands at a point of a sector, in blocks: the ground
/// in full detail, as a body stands on it.
fn ground(generator: &Generator, sector: Sector, u: f64, v: f64) -> f64 {
    let point = SurfacePoint::new(sector, u, v);
    let direction = generator.sphere().blocks().direction(point);
    generator.sample_at(direction, 0.0).height_m / BLOCK_M
}

/// The step from a cell to the next along an axis, in the world.
fn step(site: &Seated, sphere: QuadSphere, cell: [i32; 3], axis: usize) -> DVec3 {
    let mut next = cell;
    next[axis] += 1;
    site.corner(sphere, next) - site.corner(sphere, cell)
}

/// Where along an axis the pointer is read for the layer across it through
/// a cell: its face toward the eye, the low one when the eye looks up the
/// axis.
fn near(site: &Seated, sphere: QuadSphere, eye: &Eye, cell: [i32; 3], axis: usize) -> f64 {
    let (_, forward) = eye.sight();
    f64::from(cell[axis] + i32::from(step(site, sphere, cell, axis).dot(forward) < 0.0))
}

/// The axes of the three layers through a cell, in the order a hand turns a
/// stroke through them: the side it started on, then of the other two the
/// one the eye looks at more squarely, then the last.
fn turns(site: &Seated, sphere: QuadSphere, eye: &Eye, cell: [i32; 3], side: usize) -> [usize; 3] {
    let (_, forward) = eye.sight();
    let depth = |axis: usize| {
        let along = step(site, sphere, cell, axis).normalize_or_zero();
        along.dot(forward).abs()
    };
    let [p, q] = [(side + 1) % 3, (side + 2) % 3];
    match depth(p) >= depth(q) {
        true => [side, p, q],
        false => [side, q, p],
    }
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

    /// The platform a body standing at a point is given, of the side picked:
    /// the square the address cuts, its top where the slab is found.
    fn platform_at(build: &Build, point: SurfacePoint) -> Platform {
        let (x, y) = (point.u.floor() as i32, point.v.floor() as i32);
        let volumes = &build.sites[0].volumes;
        let cells = build.cells_over(point).expect("an open volume");
        let top = (cells.min[2]..=cells.max[2])
            .rev()
            .find(|&z| !volumes.get([x, y, z]).is_air())
            .expect("a slab over the column");
        Platform::over(x, y, build.platform_bits, top + 1)
    }

    /// A world with a platform of 64 cells a side laid where [`world`]
    /// stands, a whole plot of it, and the first cell over the corner of its
    /// slab: the tests speak in cells counted from there.
    fn opened() -> (Generator, Build, [i32; 3]) {
        let (generator, point) = world();
        let mut build = Build::default();
        build.set_platform(64);
        build
            .lay_platform_now(&generator, point, Base::Pillars)
            .expect("dry land");
        let platform = platform_at(&build, point);
        let origin = [platform.corner[0], platform.corner[1], platform.top];
        // The platform is there to stand on, not to take back.
        build.done.clear();
        build.drain_changes();
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
    fn stroke(build: &mut Build, generator: &Generator, from: [i32; 3], to: [i32; 3]) {
        let sphere = generator.sphere();
        let (a, b) = (top(build, sphere, from), top(build, sphere, to));
        let middle = (a + b) / 2.0;
        let mut eye = eye_at(middle + middle.normalize() * 20.0, middle);
        point_at(&mut eye, a);
        build.update(generator, &eye, true, false);
        point_at(&mut eye, b);
        build.update(generator, &eye, true, false);
        build.update(generator, &eye, false, false);
    }

    /// Every cell over the slab that is not air, in the volume the tests
    /// open, counted from the first.
    fn laid(build: &Build, origin: [i32; 3]) -> Vec<[i32; 3]> {
        let volumes = &build.sites[0].volumes;
        let plot = volumes.plot_of(origin[0], origin[1]);
        let mut over = volumes.bounds(plot).expect("an open volume");
        over.min[2] = origin[2];
        over.cells()
            .filter(|&cell| !volumes.get(cell).is_air())
            .map(|cell| [0, 1, 2].map(|i| cell[i] - origin[i]))
            .collect()
    }

    #[test]
    fn a_volume_opens_over_the_plot_under_the_body_and_leaves_the_ground() {
        let (generator, point) = world();
        let direction = generator.sphere().blocks().direction(point);
        let ground_m = generator.sample(direction).height_m;
        let mut build = Build::default();
        build.open(&generator, point).expect("dry land");
        assert!(build.covers(point));
        assert!(!build.covers(along(point, 1)));
        // The address cuts it, wherever on the plot the body stood, and it
        // holds the ground of the plot and the air over it.
        let cells = build.cells_over(point).unwrap();
        assert_eq!(cells.min[0] % 64, 0);
        assert_eq!(cells.min[1] % 64, 0);
        assert_eq!(cells.size()[0], 64);
        assert_eq!(cells.min[2] % 16, 0);
        let feet = (ground_m / BLOCK_M) as i32;
        assert!(cells.contains([point.u as i32, point.v as i32, feet]));
        assert!(cells.max[2] >= feet + HEIGHT);
        // Nature is as it was: no stamp, and nothing built.
        assert!(generator.stamps().is_empty());
        assert_eq!(generator.sample(direction).height_m, ground_m);
        assert!(build.drain_changes().is_empty());
    }

    #[test]
    fn a_plot_on_the_edge_of_a_sector_stays_nature() {
        let (generator, point) = world();
        let side = f64::from(generator.sphere().blocks().side());
        let mut build = Build::default();
        for (u, v) in [(10.0, point.v), (point.u, side - 10.0), (side - 1.0, 1.0)] {
            let edge = SurfacePoint::new(point.sector, u, v);
            let opened = build.lay_platform_now(&generator, edge, Base::Pillars);
            assert_eq!(opened.err(), Some(BuildRefusal::Seam), "{u} {v}");
        }
        // One plot in, the edge is no reason: the sea may be.
        let inside = SurfacePoint::new(point.sector, 70.0, side - 70.0);
        let opened = build.lay_platform_now(&generator, inside, Base::Pillars);
        assert_ne!(opened.err(), Some(BuildRefusal::Seam));
    }

    #[test]
    fn a_platform_is_a_slab_over_the_highest_ground_on_pillars_down_to_it() {
        let (generator, point) = world();
        let mut build = Build::default();
        build.set_platform(16);
        build.set_paint(6);
        assert_eq!(build.platform(), 16);
        let top_m = build
            .lay_platform_now(&generator, point, Base::Pillars)
            .unwrap();
        let platform = platform_at(&build, point);
        assert_eq!(f64::from(platform.top) * BLOCK_M, top_m);
        let volumes = &build.sites[0].volumes;
        // The slab is whole, in the paint in hand, and no ground under it
        // stands over its top: the highest of it is within a cell of it.
        let slab = platform.slab();
        assert_eq!(slab.size(), [16, 16, 1]);
        let mut highest = f64::MIN;
        let mut pillars = 0;
        for [x, y, z] in slab.cells() {
            assert_eq!(volumes.get([x, y, z]).paint(), Some(6), "{x} {y}");
            assert!(volumes.get([x, y, z + 1]).is_air());
            let corners = [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| {
                ground(
                    &generator,
                    point.sector,
                    f64::from(x + dx),
                    f64::from(y + dy),
                )
            });
            highest = corners.into_iter().fold(highest, f64::max);
            // Under the slab it is open, but for a pillar, which reaches
            // the lowest ground at its foot and goes no deeper.
            let foot = corners.into_iter().fold(f64::MAX, f64::min).floor() as i32;
            let below: Vec<i32> = (foot - 2..z)
                .filter(|&z| !volumes.get([x, y, z]).is_air())
                .collect();
            if !below.is_empty() {
                pillars += 1;
                assert_eq!(below, (foot..z).collect::<Vec<_>>(), "{x} {y}");
            }
        }
        assert!((0.0..1.0).contains(&(f64::from(platform.top) - highest)));
        assert!(pillars <= 4);
        // It is drawn, and one undo takes all of it back.
        let eye = generator.sphere().position(point, top_m + 2.0).into();
        build.settle(&generator, eye);
        assert!(!build.drawn.is_empty());
        assert_eq!(build.history(), (true, false));
        build.undo(&generator);
        build.settle(&generator, eye);
        assert!(build.drawn.is_empty());
        assert!(build.covers(point));
    }

    #[test]
    fn a_platform_of_64_is_drawn_over_updates_the_nearest_chunks_first() {
        let (generator, point) = world();
        let sphere = generator.sphere();
        let mut build = Build::default();
        build.set_platform(64);
        let top_m = build
            .lay_platform_now(&generator, point, Base::Solid)
            .unwrap();
        assert!(build.drawn.is_empty(), "nothing is meshed as it is laid");
        let owed = build.stale.len();
        assert!(owed > MESHES_PER_UPDATE, "{owed}");
        let eye = DVec3::from(sphere.position(point, top_m + 2.0));
        let half = CHUNK as i32 / 2;
        let far = |build: &Build, (site, chunk): (usize, [i32; 3])| {
            build.sites[site]
                .corner(sphere, chunk.map(|n| n + half))
                .distance(eye)
        };
        let mut nearest: Vec<(usize, [i32; 3])> = build.stale.iter().copied().collect();
        nearest.sort_by(|&a, &b| far(&build, a).total_cmp(&far(&build, b)));
        // The first update meshes the nearest of what is owed, and no more.
        build.mesh_owed(sphere, eye, MESHES_PER_UPDATE);
        assert!(build.drain_changes().len() <= MESHES_PER_UPDATE);
        let left: BTreeSet<_> = nearest[MESHES_PER_UPDATE..].iter().copied().collect();
        assert_eq!(build.stale, left);
        let mut updates = 1;
        while !build.settled() {
            build.mesh_owed(sphere, eye, MESHES_PER_UPDATE);
            updates += 1;
        }
        assert_eq!(updates, owed.div_ceil(MESHES_PER_UPDATE));
        assert!(!build.drawn.is_empty());
    }

    #[test]
    fn a_solid_base_fills_every_column_down_to_the_ground_on_a_steep_plot() {
        // The steepest plot found near the test world: 32 m of fall across
        // its 32 m, where a volume's bottom is most likely to cut a column.
        let (generator, point) = world();
        let side = f64::from(generator.sphere().blocks().side());
        let point = SurfacePoint::new(point.sector, side * 0.312, side * 0.372);
        let mut build = Build::default();
        build.set_platform(64);
        build
            .lay_platform_now(&generator, point, Base::Solid)
            .unwrap();
        let platform = platform_at(&build, point);
        let volumes = &build.sites[0].volumes;
        let slab = platform.slab();
        let mut deepest = 0;
        for [x, y, z] in slab.cells() {
            let foot = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .map(|(dx, dy)| {
                    ground(
                        &generator,
                        point.sector,
                        f64::from(x + dx),
                        f64::from(y + dy),
                    )
                })
                .into_iter()
                .fold(f64::MAX, f64::min)
                .floor() as i32;
            deepest = deepest.max(z - foot);
            assert!(volumes.get([x, y, foot - 1]).is_air(), "{x} {y}");
            for z in foot..=z {
                assert!(!volumes.get([x, y, z]).is_air(), "{x} {y} {z}");
            }
        }
        assert!(deepest > 50, "{deepest}");
    }

    #[test]
    fn the_side_picked_is_one_on_offer() {
        let mut build = Build::default();
        for (asked, given) in [(0, 8), (8, 8), (20, 16), (32, 32), (50, 64), (900, 64)] {
            build.set_platform(asked);
            assert_eq!(build.platform(), given, "{asked}");
        }
    }

    #[test]
    fn platforms_of_one_size_meet_edge_to_edge() {
        let (generator, point) = world();
        let mut build = Build::default();
        build.set_platform(16);
        build
            .lay_platform_now(&generator, point, Base::Pillars)
            .unwrap();
        let first = platform_at(&build, point).slab();
        let beside = SurfacePoint::new(point.sector, point.u + 16.0, point.v);
        build
            .lay_platform_now(&generator, beside, Base::Pillars)
            .unwrap();
        let second = platform_at(&build, beside).slab();
        assert_eq!(second.min[0], first.max[0] + 1);
        assert_eq!(second.min[1], first.min[1]);
    }

    #[test]
    fn a_body_on_the_platform_stands_on_its_slab() {
        let (generator, point) = world();
        let mut build = Build::default();
        let top_m = build
            .lay_platform_now(&generator, point, Base::Pillars)
            .unwrap();
        let footing = build.footing(point, top_m).unwrap();
        assert_eq!(footing.floor_m, Some(top_m));
        // The ground is under it everywhere, so it is the slab that holds.
        let feet_m = ground(&generator, point.sector, point.u, point.v) * BLOCK_M;
        assert!(feet_m <= top_m);
    }

    #[test]
    fn a_stroke_stands_over_two_neighbours() {
        let (generator, mut build, origin) = opened();
        let (_, point) = world();
        build.open(&generator, along(point, 1)).unwrap();
        build.take(Some(Tool::Create));
        build.set_paint(4);
        // From the slab out over the plot beside it, where there is none.
        stroke(
            &mut build,
            &generator,
            at(origin, [60, 20, -1]),
            at(origin, [67, 20, -1]),
        );
        let volumes = &build.sites[0].volumes;
        for x in 60..=67 {
            let cell = at(origin, [x, 20, 0]);
            assert_eq!(volumes.get(cell).paint(), Some(4), "{x}");
        }
        build.undo(&generator);
        let volumes = &build.sites[0].volumes;
        for x in 60..=67 {
            assert!(volumes.get(at(origin, [x, 20, 0])).is_air(), "{x}");
        }
    }

    #[test]
    fn a_stroke_stops_where_no_volume_stands() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            &generator,
            at(origin, [60, 20, -1]),
            at(origin, [67, 20, -1]),
        );
        let row: Vec<[i32; 3]> = (60..64).map(|x| [x, 20, 0]).collect();
        assert_eq!(laid(&build, origin), row);
    }

    #[test]
    fn a_body_stands_astride_two_volumes() {
        let (generator, mut build, origin) = opened();
        let (_, point) = world();
        build.open(&generator, along(point, 1)).unwrap();
        // One cube at the edge of the second volume, level with the slab of
        // the first, and a body half on it, its middle over the first.
        let cube = at(origin, [64, 20, -1]);
        build.lay(
            &generator,
            Gesture::Create {
                span: Span::cell(cube),
                paint: 0,
            },
        );
        let floor_m = f64::from(origin[2]) * BLOCK_M;
        let beside = |du: f64| {
            let (u, v) = (f64::from(cube[0]) + du, f64::from(cube[1]) + 0.5);
            build.footing(SurfacePoint::new(point.sector, u, v), floor_m)
        };
        assert_eq!(beside(-0.3).unwrap().floor_m, Some(floor_m));
        // Its middle over the cube, nothing else of it on anything built.
        assert_eq!(beside(0.7).unwrap().floor_m, Some(floor_m));
        // Past the cube the second volume holds nothing up.
        assert_eq!(beside(2.0).unwrap().floor_m, None);
        // And off every volume there is no footing to speak of.
        let off = SurfacePoint::new(point.sector, f64::from(origin[0]) - 5.0, point.v);
        assert_eq!(build.footing(off, floor_m), None);
    }

    #[test]
    fn a_drag_on_the_slab_lays_a_row_and_a_body_stands_on_it() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        build.set_paint(4);
        // Aimed at the slab, the new cells go on it.
        stroke(
            &mut build,
            &generator,
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
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            &generator,
            at(origin, [5, 5, -1]),
            at(origin, [7, 5, -1]),
        );
        build.take(Some(Tool::Paint));
        build.set_paint(9);
        stroke(
            &mut build,
            &generator,
            at(origin, [5, 5, 0]),
            at(origin, [5, 5, 0]),
        );
        build.take(Some(Tool::Delete));
        stroke(
            &mut build,
            &generator,
            at(origin, [7, 5, 0]),
            at(origin, [7, 5, 0]),
        );
        let volumes = &build.sites[0].volumes;
        assert_eq!(volumes.get(at(origin, [5, 5, 0])).paint(), Some(9));
        assert_eq!(volumes.get(at(origin, [6, 5, 0])).paint(), Some(0));
        assert!(volumes.get(at(origin, [7, 5, 0])).is_air());
    }

    #[test]
    fn the_slab_is_cells_like_any_other() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Delete));
        stroke(
            &mut build,
            &generator,
            at(origin, [30, 30, -1]),
            at(origin, [30, 30, -1]),
        );
        assert!(
            build.sites[0]
                .volumes
                .get(at(origin, [30, 30, -1]))
                .is_air()
        );
    }

    #[test]
    fn a_cell_the_ground_hides_is_not_under_the_pointer() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        // A cube sunk two metres into the ground, under an eye over it.
        let (x, y) = (origin[0] + 70, origin[1] + 20);
        let under = ground(
            &generator,
            build.sites[0].sector,
            f64::from(x),
            f64::from(y),
        );
        let sunk = [x, y, under.floor() as i32 - 4];
        let (_, point) = world();
        build.open(&generator, along(point, 1)).unwrap();
        build.lay(
            &generator,
            Gesture::Create {
                span: Span::cell(sunk),
                paint: 0,
            },
        );
        let over = top(&build, sphere, [x, y, under.floor() as i32 + 20]);
        let eye = eye_at(over, top(&build, sphere, sunk));
        build.update(&generator, &eye, false, false);
        assert_eq!(build.aim, None);
        assert_eq!(build.ghost(), None);
    }

    #[test]
    fn the_ghost_shows_the_stroke_and_goes_with_the_tool() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        let slab = top(&build, sphere, at(origin, [3, 3, -1]));
        let eye = eye_at(slab + slab.normalize() * 5.0, slab);
        build.update(&generator, &eye, false, false);
        assert_eq!(build.ghost(), Some(GHOST));
        build.take(None);
        build.update(&generator, &eye, false, false);
        assert_eq!(build.ghost(), None);
    }

    #[test]
    fn a_click_lays_one_cell_however_low_the_eye() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        let slab = top(&build, sphere, at(origin, [20, 20, -1]));
        let site = &build.sites[0];
        let back = site.corner(sphere, at(origin, [14, 20, 0]))
            - site.corner(sphere, at(origin, [20, 20, 0]));
        // Low and far: a plane read half a cell up would land cells away.
        let mut eye = eye_at(slab + back + slab.normalize() * 0.8, slab);
        point_at(&mut eye, slab);
        build.update(&generator, &eye, true, false);
        build.update(&generator, &eye, false, false);
        assert_eq!(laid(&build, origin), vec![[20, 20, 0]]);
    }

    /// A low eye eight cells back from a cell of the slab, the pointer on
    /// it, that presses, moves the pointer up the view and lets go.
    fn up_the_view(
        build: &mut Build,
        generator: &Generator,
        origin: [i32; 3],
        upright: bool,
    ) -> Vec<[i32; 3]> {
        let sphere = generator.sphere();
        let slab = top(build, sphere, at(origin, [20, 20, -1]));
        let site = &build.sites[0];
        let back = site.corner(sphere, at(origin, [12, 20, 0]))
            - site.corner(sphere, at(origin, [20, 20, 0]));
        let mut eye = eye_at(slab + back + slab.normalize() * 2.0, slab);
        point_at(&mut eye, slab);
        build.update(generator, &eye, true, upright);
        eye.pointer[1] -= 0.2;
        build.update(generator, &eye, true, upright);
        build.update(generator, &eye, false, upright);
        laid(build, origin)
    }

    #[test]
    fn a_drag_up_the_view_lays_a_slab_away_from_the_eye() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        let laid = up_the_view(&mut build, &generator, origin, false);
        assert!(laid.len() >= 3, "{laid:?}");
        assert!(laid.iter().all(|at| at[2] == 0 && at[1] == 20), "{laid:?}");
    }

    /// The middle of a side of a cell.
    fn side(build: &Build, sphere: QuadSphere, cell: [i32; 3], face: voxel::Face) -> DVec3 {
        let quad = Quad {
            cell,
            face,
            paint: 0,
            open: [3; 4],
        };
        let site = &build.sites[0];
        let corners = quad.corners().map(|corner| site.corner(sphere, corner));
        corners.into_iter().sum::<DVec3>() / 4.0
    }

    /// Cells laid on the slab, in the paint of the platform.
    fn stand(build: &mut Build, generator: &Generator, a: [i32; 3], b: [i32; 3]) {
        build.lay(
            generator,
            Gesture::Create {
                span: Span::between(a, b),
                paint: 0,
            },
        );
    }

    #[test]
    fn from_the_foot_of_a_wall_a_stroke_goes_up_the_wall() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        stand(
            &mut build,
            &generator,
            at(origin, [30, 10, 0]),
            at(origin, [30, 20, 5]),
        );
        build.take(Some(Tool::Create));
        build.set_paint(4);
        // From the slab at the foot of the wall, onto the wall itself.
        let foot = top(&build, sphere, at(origin, [29, 15, -1]));
        let wall = side(
            &build,
            sphere,
            at(origin, [30, 12, 4]),
            voxel::Face::new(0, false),
        );
        let back = build.sites[0].corner(sphere, at(origin, [14, 14, 0]))
            - build.sites[0].corner(sphere, at(origin, [29, 14, 0]));
        let mut eye = eye_at(foot + back + foot.normalize() * 6.0, foot);
        point_at(&mut eye, foot);
        build.update(&generator, &eye, true, false);
        point_at(&mut eye, wall);
        build.update(&generator, &eye, true, false);
        build.update(&generator, &eye, false, false);
        // A sheet against the wall, from the slab up to where it pointed.
        let sheet: Vec<[i32; 3]> = laid(&build, origin)
            .into_iter()
            .filter(|cell| cell[0] != 30)
            .collect();
        assert_eq!(sheet.len(), 4 * 5, "{sheet:?}");
        assert!(sheet.iter().all(|cell| cell[0] == 29), "{sheet:?}");
        assert!(sheet.contains(&[29, 12, 4]) && sheet.contains(&[29, 15, 0]));
    }

    #[test]
    fn a_hand_turns_a_stroke_through_the_three_layers() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        build.take(Some(Tool::Create));
        let from = top(&build, sphere, at(origin, [20, 20, -1]));
        let to = top(&build, sphere, at(origin, [24, 23, -1]));
        let mut eye = eye_at(from + from.normalize() * 20.0, from);
        point_at(&mut eye, from);
        build.update(&generator, &eye, true, false);
        point_at(&mut eye, to);
        let mut across = Vec::new();
        for turn in [false, true, false, true, false, true] {
            build.update(&generator, &eye, true, turn);
            let stroke = build.stroke.expect("a stroke being drawn");
            // Whichever way it lies, it is one cell thick, through its start.
            let (axis, _) = stroke.across;
            assert_eq!(stroke.end[axis], stroke.start[axis]);
            across.push(axis);
        }
        // On the slab it lies flat; turned, it stands one way, then the
        // other, and then lies flat again.
        assert_eq!(across[0], 2);
        assert_eq!(across[1], across[2]);
        assert_eq!(across[3], across[4]);
        assert_eq!(across[5], 2);
        let mut stood = [across[1], across[3]];
        stood.sort_unstable();
        assert_eq!(stood, [0, 1]);
    }

    #[test]
    fn turned_a_stroke_from_the_side_of_a_pillar_lies_flat() {
        let (generator, mut build, origin) = opened();
        let sphere = generator.sphere();
        stand(
            &mut build,
            &generator,
            at(origin, [30, 30, 0]),
            at(origin, [30, 30, 6]),
        );
        build.take(Some(Tool::Create));
        build.set_paint(4);
        let face = voxel::Face::new(0, false);
        let from = side(&build, sphere, at(origin, [30, 30, 4]), face);
        // From high over it, the eye looks at the flat layer most squarely.
        let out = build.sites[0].corner(sphere, at(origin, [22, 30, 0]))
            - build.sites[0].corner(sphere, at(origin, [30, 30, 0]));
        let mut eye = eye_at(from + out + from.normalize() * 16.0, from);
        point_at(&mut eye, from);
        build.update(&generator, &eye, true, false);
        // Left alone it hugs the side it started on: a sheet, standing.
        let up = side(&build, sphere, at(origin, [29, 33, 6]), face);
        point_at(&mut eye, up);
        build.update(&generator, &eye, true, false);
        assert_eq!(build.stroke.unwrap().across.0, 0);
        // Turned once, it lies flat at the height it started at.
        let away = top(&build, sphere, at(origin, [25, 33, 4]));
        point_at(&mut eye, away);
        build.update(&generator, &eye, true, true);
        build.update(&generator, &eye, false, true);
        let shelf: Vec<[i32; 3]> = laid(&build, origin)
            .into_iter()
            .filter(|cell| cell[0] != 30)
            .collect();
        assert_eq!(shelf.len(), 5 * 4, "{shelf:?}");
        assert!(shelf.iter().all(|cell| cell[2] == 4), "{shelf:?}");
        assert!(shelf.contains(&[29, 30, 4]) && shelf.contains(&[25, 33, 4]));
        // The next stroke lies on its side again.
        build.update(&generator, &eye, false, false);
        assert_eq!(build.turned, 0);
    }

    #[test]
    fn upright_the_same_drag_stands_a_wall() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        let laid = up_the_view(&mut build, &generator, origin, true);
        assert!(laid.len() >= 3, "{laid:?}");
        assert!(laid.iter().all(|at| at[0] == 20 && at[1] == 20), "{laid:?}");
        assert_eq!(laid.iter().map(|at| at[2]).min(), Some(0));
    }

    #[test]
    fn undo_takes_back_a_stroke_and_redo_puts_it_back() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            &generator,
            at(origin, [5, 5, -1]),
            at(origin, [7, 5, -1]),
        );
        let middle = at(origin, [6, 5, 0]);
        assert_eq!(build.history(), (true, false));
        build.undo(&generator);
        assert!(build.sites[0].volumes.get(middle).is_air());
        assert_eq!(build.history(), (false, true));
        build.redo(&generator);
        assert!(!build.sites[0].volumes.get(middle).is_air());
        // A new stroke forgets what was taken back.
        build.undo(&generator);
        stroke(
            &mut build,
            &generator,
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
        let slab = top(&build, sphere, at(origin, [3, 3, -1]));
        let eye = eye_at(slab + slab.normalize() * 5.0, slab);
        build.update(&generator, &eye, true, false);
        assert!(build.cancel());
        build.update(&generator, &eye, false, false);
        assert!(build.sites[0].volumes.get(at(origin, [3, 3, 0])).is_air());
        assert!(!build.cancel());
    }

    #[test]
    fn the_ghost_is_meshed_again_after_a_stroke_lands() {
        let (generator, mut build, origin) = opened();
        build.take(Some(Tool::Create));
        stroke(
            &mut build,
            &generator,
            at(origin, [3, 3, -1]),
            at(origin, [3, 3, -1]),
        );
        build.take(Some(Tool::Delete));
        build.drain_changes();
        stroke(
            &mut build,
            &generator,
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
            volumes: Volumes::new(PLOT_BITS),
            balls: Vec::new(),
        };
        let chunks = [[64, 128, 32], [80, 128, 32], [64, 144, 32], [64, 128, -32]];
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
