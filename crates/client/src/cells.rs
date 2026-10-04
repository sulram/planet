//! The cells of a world on the client: volumes seated on the planet, what
//! changes them, and the meshes that draw them (DECISIONS 106).
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
//! The ground under a volume stays as nature made it (DECISIONS 78).
//!
//! The cells are a system of the core (DECISIONS 108): anyone reads what
//! they hold, and a gesture alone changes it. How a hand arrives at gestures,
//! a tool, a stroke, a platform, is a plugin's, and it speaks to the cells
//! through its host (`crate::plugin`): aim, preview, apply, take back.
//!
//! Volumes live here only: nothing is sent or kept yet (DECISIONS 76).

use std::collections::BTreeSet;

use glam::{DVec3, Vec3};
use scene::{VolumeChange, VolumeMesh, VolumeMeshId, VolumeVertex};
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint};
use voxel::{CHUNK, CHUNK_BITS, Cell, Gesture, Hit, Quad, Span, Volumes, crossing};
use worldgen::Generator;

use crate::collision::{Footing, STEP_M};
use crate::grass::TUFT_M;

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
/// How far the ghost of a gesture stands off the cubes it covers, metres.
const GHOST_LIFT_M: f32 = 0.012;
/// The ghost's id, which no chunk of any volume has.
const GHOST: VolumeMeshId = VolumeMeshId(u64::MAX);
/// Changes kept to take back.
const HISTORY: usize = 100;
/// Chunks meshed in one update, the nearest the eye first. A change to more
/// is drawn over as many updates as it takes, each chunk keeping the mesh it
/// had until its new one is made, so no frame pays for a platform whole.
const MESHES_PER_UPDATE: usize = 8;

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

/// What a gesture that deletes shows over what it takes away.
const DELETE_COLOR: [u8; 3] = [230, 70, 60];

/// Why the cells refuse what was asked of them. A plugin says it at the
/// seam under its own name, and a front end in its own words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// Changing the cells is a builder's and an admin's, and the world named
    /// this session a lower level, or no world has spoken (DECISIONS 104).
    Level,
    /// The ground here is under the sea.
    Sea,
    /// The plot is on the edge of a sector: a volume stays inside one.
    Seam,
}

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

/// What cells are seated on: the frame their addresses are counted in. A
/// sector of the planet today. A body that moves, a ship with a room aboard,
/// is the next kind (DECISIONS 108), so every word of the cells says where
/// as a seat and never as a bare address.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Seat {
    Sector(Sector),
}

impl From<Sector> for Seat {
    fn from(sector: Sector) -> Seat {
        Seat::Sector(sector)
    }
}

/// What a line of sight meets: a side of a cell, in the cells of a seat.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Aim {
    pub seat: Seat,
    pub hit: Hit,
}

/// A line of sight bent into the cells of every seat built on, and what it
/// meets first.
pub struct Sight {
    paths: Vec<(Seat, Vec<[f64; 3]>)>,
    aim: Option<Aim>,
}

impl Sight {
    /// The nearest cell it meets, over any seat, unless the ground stands
    /// between it and where it starts: a cell a hill hides is not in sight.
    pub fn aim(&self) -> Option<Aim> {
        self.aim
    }

    /// Where it crosses the layer of a seat's cells across `axis` at `at`,
    /// whatever is behind, and over as many volumes as it reaches.
    pub fn crossing(&self, seat: impl Into<Seat>, axis: usize, at: f64) -> Option<[f64; 3]> {
        let seat = seat.into();
        let (_, path) = self.paths.iter().find(|(on, _)| *on == seat)?;
        crossing(path, axis, at)
    }
}

/// A change that landed, as the cells of its box before and after.
struct Change {
    site: usize,
    span: Span,
    before: Vec<Cell>,
    after: Vec<Cell>,
}

/// Every volume of a world, what is drawn of them and what was changed.
#[derive(Default)]
pub struct Cells {
    /// The sectors a volume stands on, in the order the first one opened.
    sites: Vec<Seated>,
    /// What the ghost shows now: it is meshed again only when that changes.
    ghost: Option<(Seat, Gesture)>,
    drawn: BTreeSet<VolumeMeshId>,
    /// Chunks owed a mesh since their cells changed, by site.
    stale: BTreeSet<(usize, [i32; 3])>,
    /// Whether a change of many gestures landed since the last update.
    landed: bool,
    changes: Vec<VolumeChange>,
    /// Where cells changed near enough the ground to cover or bare a tuft,
    /// as a cap of the body: its middle, unit, and its angle.
    touched: Vec<(DVec3, f64)>,
    /// Changes to take back, the last one last, and those taken back.
    done: Vec<Change>,
    undone: Vec<Change>,
}

impl Cells {
    /// The volumes of a seat, when something is built on it.
    fn site(&self, seat: Seat) -> Option<usize> {
        let Seat::Sector(sector) = seat;
        self.sites.iter().position(|site| site.sector == sector)
    }

    /// Whether a volume stands on a column.
    pub fn covers(&self, point: SurfacePoint) -> bool {
        self.site(point.sector.into())
            .is_some_and(|site| self.sites[site].volumes.is_open(plot_of(point)))
    }

    /// The cells the volume over a column holds.
    pub fn bounds_over(&self, point: SurfacePoint) -> Option<Span> {
        self.sites[self.site(point.sector.into())?]
            .volumes
            .bounds(plot_of(point))
    }

    /// Whether a volume holds a cell of a seat, air or not: where a gesture
    /// can land.
    pub fn holds(&self, seat: impl Into<Seat>, cell: [i32; 3]) -> bool {
        self.site(seat.into())
            .is_some_and(|site| self.sites[site].volumes.holds(cell))
    }

    /// What a cell of a seat is. Air where no volume holds it.
    pub fn cell(&self, seat: impl Into<Seat>, at: [i32; 3]) -> Cell {
        self.site(seat.into())
            .map_or_else(Cell::default, |site| self.sites[site].volumes.get(at))
    }

    /// Opens the volume of the plot a column is on, where none stands. The
    /// ground stays as it is, and the volume holds the blocks from the lowest
    /// of it to [`HEIGHT`] over the highest.
    pub(crate) fn open(
        &mut self,
        generator: &Generator,
        point: SurfacePoint,
    ) -> Result<(), Refusal> {
        let sphere = generator.sphere();
        let plot = plot_of(point);
        // The corners of a sector are nature, and a build does not fold over
        // a seam: a plot on the edge of its sector stays as it is.
        let inside = sphere
            .blocks()
            .coarsened(PLOT_BITS)
            .is_some_and(|plots| plot.iter().all(|&n| 0 < n && n < plots.side() as i32 - 1));
        if !inside {
            return Err(Refusal::Seam);
        }
        if ground(generator, point) < 0.0 {
            return Err(Refusal::Sea);
        }
        if self.covers(point) {
            return Ok(());
        }
        let low = plot.map(|n| n << PLOT_BITS);
        let side = 1 << PLOT_BITS;
        let heights = (0..=side).step_by(SURVEY as usize).flat_map(|dv| {
            (0..=side).step_by(SURVEY as usize).map(move |du| {
                let (u, v) = (f64::from(low[0] + du), f64::from(low[1] + dv));
                ground(generator, SurfacePoint::new(point.sector, u, v))
            })
        });
        let (lowest, highest) = heights.fold((f64::MAX, f64::MIN), |(lo, hi), blocks| {
            (lo.min(blocks), hi.max(blocks))
        });
        let bottom = lowest.floor() as i32 - UNDER;
        let top = highest.ceil() as i32 + HEIGHT;
        let site = match self.site(point.sector.into()) {
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

    /// Forgets every volume: another world.
    pub(crate) fn clear(&mut self) {
        for id in core::mem::take(&mut self.drawn) {
            self.changes.push(VolumeChange::Remove(id));
        }
        if self.ghost.take().is_some() {
            self.changes.push(VolumeChange::Remove(GHOST));
        }
        self.sites.clear();
        self.stale.clear();
        self.landed = false;
        self.touched.clear();
        self.done.clear();
        self.undone.clear();
    }

    /// Whether there is a change to take back, and one to put back.
    pub fn history(&self) -> (bool, bool) {
        (!self.done.is_empty(), !self.undone.is_empty())
    }

    /// Takes back the last change that landed.
    pub(crate) fn take_back(&mut self, generator: &Generator) {
        if let Some(change) = self.done.pop() {
            let changed = self.sites[change.site]
                .volumes
                .restore(change.span, &change.before);
            if let Some(changed) = changed {
                self.redraw(generator, change.site, changed);
            }
            self.undone.push(change);
        }
    }

    /// Puts back the last change taken back.
    pub(crate) fn put_back(&mut self, generator: &Generator) {
        if let Some(change) = self.undone.pop() {
            let changed = self.sites[change.site]
                .volumes
                .restore(change.span, &change.after);
            if let Some(changed) = changed {
                self.redraw(generator, change.site, changed);
            }
            self.done.push(change);
        }
    }

    /// One frame of the picture: meshes a few of the chunks owed one, the
    /// nearest the eye first. A change of many gestures is the work of the
    /// update it landed in, and its meshes start with the next.
    pub(crate) fn update(&mut self, sphere: QuadSphere, eye: DVec3) {
        if !core::mem::take(&mut self.landed) {
            self.mesh_owed(sphere, eye, MESHES_PER_UPDATE);
        }
    }

    /// Whether every chunk that changed is drawn as it is now.
    pub fn settled(&self) -> bool {
        self.stale.is_empty()
    }

    /// Meshes every chunk owed one: for a picture that must show all of it.
    pub(crate) fn settle(&mut self, sphere: QuadSphere, eye: DVec3) {
        self.landed = false;
        self.mesh_owed(sphere, eye, usize::MAX);
    }

    /// A line of sight from a point of the world along a direction, unit,
    /// and what it meets.
    pub fn sight(&self, generator: &Generator, from: DVec3, toward: DVec3) -> Sight {
        let sphere = generator.sphere();
        let paths: Vec<(Seat, Vec<[f64; 3]>)> = self
            .sites
            .iter()
            .map(|site| (site.sector.into(), site.sight(sphere, from, toward)))
            .collect();
        let met = self
            .sites
            .iter()
            .zip(&paths)
            .filter_map(|(site, (seat, path))| {
                let hit = site.volumes.trace(path)?;
                let middle = site.corner(sphere, hit.cell);
                let aim = Aim { seat: *seat, hit };
                Some(((middle - from).dot(toward), aim))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let aim = met.and_then(|(away_m, aim)| {
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
        });
        Sight { paths, aim }
    }

    /// Where a lattice point of a seat's cells is in the world.
    pub fn corner(&self, sphere: QuadSphere, seat: impl Into<Seat>, p: [i32; 3]) -> DVec3 {
        let Seat::Sector(sector) = seat.into();
        let point = SurfacePoint::new(sector, f64::from(p[0]), f64::from(p[1]));
        DVec3::from(sphere.position(point, f64::from(p[2]) * BLOCK_M))
    }

    /// Applies gestures to the cells of a seat as one change, kept to take
    /// back as one, and draws what it changed. False when no cell changed.
    pub(crate) fn apply(
        &mut self,
        generator: &Generator,
        seat: impl Into<Seat>,
        gestures: &[Gesture],
    ) -> bool {
        let Some(site) = self.site(seat.into()) else {
            return false;
        };
        let reach = gestures
            .iter()
            .map(|gesture| gesture.span())
            .reduce(|a, b| a.with(b.min).with(b.max));
        let volumes = &mut self.sites[site].volumes;
        // What is kept is what volumes hold of the change, however far it ran.
        let Some(span) = reach.and_then(|reach| volumes.held(reach)) else {
            return false;
        };
        let before = volumes.cells(span);
        let changed = gestures
            .iter()
            .filter_map(|&gesture| volumes.apply(gesture))
            .reduce(|a, b| a.with(b.min).with(b.max));
        let Some(changed) = changed else {
            return false;
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
        self.landed |= gestures.len() > 1;
        self.redraw(generator, site, changed);
        true
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
            .map(|[u, v]| ground(generator, SurfacePoint::new(seated.sector, u, v)))
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

    /// Shows the ghost of a gesture, or of none: exactly the cells it would
    /// change, so a hand sees what it will do. It is meshed again only when
    /// what it shows is another gesture, or the cells under it changed.
    pub(crate) fn preview(&mut self, sphere: QuadSphere, gesture: Option<(Seat, Gesture)>) {
        let wanted = gesture.filter(|&(seat, _)| self.site(seat).is_some());
        if wanted == self.ghost {
            return;
        }
        self.ghost = wanted;
        let seated = wanted.and_then(|(seat, gesture)| Some((self.site(seat)?, gesture)));
        let Some((site, gesture)) = seated else {
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
        let site = &self.sites[self.site(point.sector.into())?];
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
        let Some(site) = self.site(sector.into()) else {
            return false;
        };
        let [x, y] = column.map(|at| at.floor() as i32);
        let [from, to] = heights.map(|m| m / BLOCK_M);
        self.sites[site].volumes.covers(x, y, from, to)
    }

    /// Mesh uploads and removals since the last call, in order.
    pub(crate) fn drain_changes(&mut self) -> Vec<VolumeChange> {
        core::mem::take(&mut self.changes)
    }

    /// Where the grass is to grow again since the last call: caps of the
    /// body, each a middle, unit, and an angle.
    pub(crate) fn drain_touched(&mut self) -> Vec<(DVec3, f64)> {
        core::mem::take(&mut self.touched)
    }

    /// Every volume mesh to draw.
    pub fn drawn(&self) -> Vec<VolumeMeshId> {
        self.drawn.iter().copied().collect()
    }

    /// The ghost of the gesture a hand would make, while there is one.
    pub fn ghost(&self) -> Option<VolumeMeshId> {
        self.ghost.map(|_| GHOST)
    }
}

/// The plot a column is on: its address, less the bits of a plot.
fn plot_of(point: SurfacePoint) -> [i32; 2] {
    [point.u, point.v].map(|at| at.floor() as i32 >> PLOT_BITS)
}

/// How high the ground stands at a point, in blocks: the ground in full
/// detail, as a body stands on it.
pub(crate) fn ground(generator: &Generator, point: SurfacePoint) -> f64 {
    let direction = generator.sphere().blocks().direction(point);
    generator.sample_at(direction, 0.0).height_m / BLOCK_M
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
impl Cells {
    /// Opens the volume over a column and lays a floor across its plot, one
    /// cell thick, over the highest ground under it. Returns the first cell
    /// over the corner of the floor: tests speak in cells counted from there.
    pub(crate) fn floor(&mut self, generator: &Generator, point: SurfacePoint) -> [i32; 3] {
        self.open(generator, point).expect("dry land");
        let [x, y] = plot_of(point).map(|n| n << PLOT_BITS);
        let side = 1 << PLOT_BITS;
        let highest = (0..=side)
            .flat_map(|dy| (0..=side).map(move |dx| (dx, dy)))
            .map(|(dx, dy)| {
                let (u, v) = (f64::from(x + dx), f64::from(y + dy));
                ground(generator, SurfacePoint::new(point.sector, u, v))
            })
            .fold(f64::MIN, f64::max);
        let top = highest.ceil() as i32;
        let far = side - 1;
        let span = Span::between([x, y, top - 1], [x + far, y + far, top - 1]);
        self.apply(
            generator,
            point.sector,
            &[Gesture::Create { span, paint: 0 }],
        );
        // The floor is there to stand on, not to take back.
        self.done.clear();
        self.changes.clear();
        [x, y, top]
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

    /// A world with a floor laid across the plot where [`world`] stands, and
    /// the first cell over its corner.
    fn opened() -> (Generator, Cells, [i32; 3]) {
        let (generator, point) = world();
        let mut cells = Cells::default();
        let origin = cells.floor(&generator, point);
        (generator, cells, origin)
    }

    /// A column `plots` plots along `u` from another.
    fn along(point: SurfacePoint, plots: i32) -> SurfacePoint {
        let blocks = f64::from(plots << PLOT_BITS);
        SurfacePoint::new(point.sector, point.u + blocks, point.v)
    }

    fn at(origin: [i32; 3], cell: [i32; 3]) -> [i32; 3] {
        [0, 1, 2].map(|i| origin[i] + cell[i])
    }

    fn create(a: [i32; 3], b: [i32; 3], paint: u8) -> Gesture {
        Gesture::Create {
            span: Span::between(a, b),
            paint,
        }
    }

    /// The middle of the top of a cell.
    fn top(cells: &Cells, sphere: QuadSphere, cell: [i32; 3]) -> DVec3 {
        let site = &cells.sites[0];
        let low = site.corner(sphere, [cell[0], cell[1], cell[2] + 1]);
        let high = site.corner(sphere, [cell[0] + 1, cell[1] + 1, cell[2] + 1]);
        (low + high) / 2.0
    }

    /// Every cell over the floor that is not air, in the volume the tests
    /// open, counted from the first.
    fn laid(cells: &Cells, origin: [i32; 3]) -> Vec<[i32; 3]> {
        let volumes = &cells.sites[0].volumes;
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
        let mut cells = Cells::default();
        cells.open(&generator, point).expect("dry land");
        assert!(cells.covers(point));
        assert!(!cells.covers(along(point, 1)));
        // The address cuts it, wherever on the plot the body stood, and it
        // holds the ground of the plot and the air over it.
        let held = cells.bounds_over(point).unwrap();
        assert_eq!(held.min[0] % 64, 0);
        assert_eq!(held.min[1] % 64, 0);
        assert_eq!(held.size()[0], 64);
        assert_eq!(held.min[2] % 16, 0);
        let feet = (ground_m / BLOCK_M) as i32;
        assert!(held.contains([point.u as i32, point.v as i32, feet]));
        assert!(held.max[2] >= feet + HEIGHT);
        // Nature is as it was: no stamp, and nothing built.
        assert!(generator.stamps().is_empty());
        assert_eq!(generator.sample(direction).height_m, ground_m);
        assert!(cells.drain_changes().is_empty());
    }

    #[test]
    fn a_plot_on_the_edge_of_a_sector_stays_nature() {
        let (generator, point) = world();
        let side = f64::from(generator.sphere().blocks().side());
        let mut cells = Cells::default();
        for (u, v) in [(10.0, point.v), (point.u, side - 10.0), (side - 1.0, 1.0)] {
            let edge = SurfacePoint::new(point.sector, u, v);
            assert_eq!(cells.open(&generator, edge), Err(Refusal::Seam), "{u} {v}");
        }
        // One plot in, the edge is no reason: the sea may be.
        let inside = SurfacePoint::new(point.sector, 70.0, side - 70.0);
        assert_ne!(cells.open(&generator, inside), Err(Refusal::Seam));
    }

    #[test]
    fn a_change_of_many_chunks_is_drawn_over_updates_the_nearest_first() {
        let (generator, mut cells, origin) = opened();
        let sphere = generator.sphere();
        let sector = cells.sites[0].sector;
        cells.settle(sphere, DVec3::ZERO);
        cells.drain_changes();
        // Two blocks the width of the plot, sixteen cells high.
        let blocks = [
            create(at(origin, [0, 0, 0]), at(origin, [63, 31, 15]), 1),
            create(at(origin, [0, 32, 0]), at(origin, [63, 63, 15]), 2),
        ];
        assert!(cells.apply(&generator, sector, &blocks));
        let owed = cells.stale.len();
        assert!(owed > MESHES_PER_UPDATE, "{owed}");
        let eye = top(&cells, sphere, at(origin, [20, 20, 20]));
        let half = CHUNK as i32 / 2;
        let far = |cells: &Cells, (site, chunk): (usize, [i32; 3])| {
            cells.sites[site]
                .corner(sphere, chunk.map(|n| n + half))
                .distance(eye)
        };
        let mut nearest: Vec<(usize, [i32; 3])> = cells.stale.iter().copied().collect();
        nearest.sort_by(|&a, &b| far(&cells, a).total_cmp(&far(&cells, b)));
        // The update it landed in meshes nothing of it.
        cells.update(sphere, eye);
        assert!(cells.drain_changes().is_empty());
        assert_eq!(cells.stale.len(), owed);
        // The next meshes the nearest of what is owed, and no more.
        cells.update(sphere, eye);
        assert!(cells.drain_changes().len() <= MESHES_PER_UPDATE);
        let left: BTreeSet<_> = nearest[MESHES_PER_UPDATE..].iter().copied().collect();
        assert_eq!(cells.stale, left);
        let mut updates = 1;
        while !cells.settled() {
            cells.update(sphere, eye);
            updates += 1;
        }
        assert_eq!(updates, owed.div_ceil(MESHES_PER_UPDATE));
        assert!(!cells.drawn.is_empty());
    }

    #[test]
    fn a_gesture_stands_over_two_neighbours_and_is_taken_back_as_one() {
        let (generator, mut cells, origin) = opened();
        let (_, point) = world();
        cells.open(&generator, along(point, 1)).unwrap();
        // From the floor out over the plot beside it, where there is none.
        let row = create(at(origin, [60, 20, 0]), at(origin, [67, 20, 0]), 4);
        assert!(cells.apply(&generator, point.sector, &[row]));
        for x in 60..=67 {
            let cell = cells.cell(point.sector, at(origin, [x, 20, 0]));
            assert_eq!(cell.paint(), Some(4), "{x}");
        }
        cells.take_back(&generator);
        for x in 60..=67 {
            assert!(cells.cell(point.sector, at(origin, [x, 20, 0])).is_air());
        }
    }

    #[test]
    fn a_gesture_stops_where_no_volume_stands() {
        let (generator, mut cells, origin) = opened();
        let sector = cells.sites[0].sector;
        let row = create(at(origin, [60, 20, 0]), at(origin, [67, 20, 0]), 0);
        assert!(cells.apply(&generator, sector, &[row]));
        let held: Vec<[i32; 3]> = (60..64).map(|x| [x, 20, 0]).collect();
        assert_eq!(laid(&cells, origin), held);
        assert!(cells.holds(sector, at(origin, [63, 20, 0])));
        assert!(!cells.holds(sector, at(origin, [64, 20, 0])));
        // A sector nothing is built on holds nothing, and takes nothing.
        let other = Sector::new(1).unwrap();
        assert!(!cells.apply(&generator, other, &[row]));
        assert!(cells.cell(other, at(origin, [60, 20, 0])).is_air());
    }

    #[test]
    fn a_body_stands_astride_two_volumes() {
        let (generator, mut cells, origin) = opened();
        let (_, point) = world();
        cells.open(&generator, along(point, 1)).unwrap();
        // One cube at the edge of the second volume, level with the floor of
        // the first, and a body half on it, its middle over the first.
        let cube = at(origin, [64, 20, -1]);
        cells.apply(&generator, point.sector, &[create(cube, cube, 0)]);
        let floor_m = f64::from(origin[2]) * BLOCK_M;
        let beside = |du: f64| {
            let (u, v) = (f64::from(cube[0]) + du, f64::from(cube[1]) + 0.5);
            cells.footing(SurfacePoint::new(point.sector, u, v), floor_m)
        };
        assert_eq!(beside(-0.3).unwrap().floor_m, Some(floor_m));
        // Its middle over the cube, nothing else of it on anything built.
        assert_eq!(beside(0.7).unwrap().floor_m, Some(floor_m));
        // Past the cube the second volume holds nothing up.
        assert_eq!(beside(2.0).unwrap().floor_m, None);
        // And off every volume there is no footing to speak of.
        let off = SurfacePoint::new(point.sector, f64::from(origin[0]) - 5.0, point.v);
        assert_eq!(cells.footing(off, floor_m), None);
    }

    #[test]
    fn a_body_stands_on_what_is_laid() {
        let (generator, mut cells, origin) = opened();
        let sphere = generator.sphere();
        let sector = cells.sites[0].sector;
        let row = create(at(origin, [10, 20, 0]), at(origin, [14, 20, 0]), 4);
        cells.apply(&generator, sector, &[row]);
        // One gesture is drawn in the update it landed in.
        cells.update(sphere, top(&cells, sphere, at(origin, [12, 20, 8])));
        assert!(!cells.drawn.is_empty());
        let on = SurfacePoint::new(
            sector,
            f64::from(origin[0]) + 12.5,
            f64::from(origin[1]) + 20.5,
        );
        let footing = cells.footing(on, f64::from(origin[2]) * BLOCK_M).unwrap();
        assert_eq!(footing.floor_m, Some(f64::from(origin[2] + 1) * BLOCK_M));
    }

    #[test]
    fn a_cell_the_ground_hides_is_not_under_the_pointer() {
        let (generator, mut cells, origin) = opened();
        let sphere = generator.sphere();
        let (_, point) = world();
        // A cube sunk two metres into the ground, under an eye over it.
        let (x, y) = (origin[0] + 70, origin[1] + 20);
        let column = SurfacePoint::new(point.sector, f64::from(x), f64::from(y));
        let under = ground(&generator, column).floor() as i32;
        let sunk = [x, y, under - 4];
        cells.open(&generator, along(point, 1)).unwrap();
        cells.apply(&generator, point.sector, &[create(sunk, sunk, 0)]);
        let over = top(&cells, sphere, [x, y, under + 20]);
        let down = (top(&cells, sphere, sunk) - over).normalize();
        assert_eq!(cells.sight(&generator, over, down).aim(), None);
        // The floor beside it is in plain sight.
        let seen = at(origin, [30, 20, -1]);
        let floor = top(&cells, sphere, seen);
        let over = floor + floor.normalize() * 5.0;
        let sight = cells.sight(&generator, over, -floor.normalize());
        let aim = sight.aim().expect("the floor");
        assert_eq!((aim.seat, aim.hit.cell), (point.sector.into(), seen));
        // The address says where a corner of a cell is, as its sides are drawn.
        let corner = cells.corner(sphere, aim.seat, seen);
        assert_eq!(corner, cells.sites[0].corner(sphere, seen));
    }

    #[test]
    fn the_ghost_shows_a_gesture_and_goes() {
        let (generator, mut cells, origin) = opened();
        let sphere = generator.sphere();
        let sector = cells.sites[0].sector;
        let cube = at(origin, [3, 3, 0]);
        cells.preview(sphere, Some((sector.into(), create(cube, cube, 0))));
        assert_eq!(cells.ghost(), Some(GHOST));
        cells.preview(sphere, None);
        assert_eq!(cells.ghost(), None);
        // Over a sector nothing is built on there is nothing to show.
        let other = Sector::new(1).unwrap();
        cells.preview(sphere, Some((other.into(), create(cube, cube, 0))));
        assert_eq!(cells.ghost(), None);
    }

    #[test]
    fn the_ghost_is_meshed_again_after_a_change_lands() {
        let (generator, mut cells, origin) = opened();
        let sphere = generator.sphere();
        let sector = cells.sites[0].sector;
        let cube = at(origin, [3, 3, 0]);
        cells.apply(&generator, sector, &[create(cube, cube, 0)]);
        let delete = Gesture::Delete {
            span: Span::cell(cube),
        };
        cells.preview(sphere, Some((sector.into(), delete)));
        cells.drain_changes();
        cells.apply(&generator, sector, &[delete]);
        cells.preview(sphere, Some((sector.into(), delete)));
        // Whatever the last frame shows over the hole, it is not the cube.
        let ghost = cells
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
    fn a_change_is_taken_back_and_put_back() {
        let (generator, mut cells, origin) = opened();
        let sector = cells.sites[0].sector;
        let row = create(at(origin, [5, 5, 0]), at(origin, [7, 5, 0]), 0);
        cells.apply(&generator, sector, &[row]);
        let middle = at(origin, [6, 5, 0]);
        assert_eq!(cells.history(), (true, false));
        cells.take_back(&generator);
        assert!(cells.cell(sector, middle).is_air());
        assert_eq!(cells.history(), (false, true));
        cells.put_back(&generator);
        assert!(!cells.cell(sector, middle).is_air());
        // A new change forgets what was taken back.
        cells.take_back(&generator);
        let cube = at(origin, [9, 9, 0]);
        cells.apply(&generator, sector, &[create(cube, cube, 0)]);
        assert_eq!(cells.history(), (true, false));
        // What changes nothing is no change to take back.
        cells.take_back(&generator);
        assert!(!cells.apply(&generator, sector, &[]));
        let air = Gesture::Delete {
            span: Span::cell(cube),
        };
        assert!(!cells.apply(&generator, sector, &[air]));
        assert_eq!(cells.history(), (false, true));
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
        let (generator, cells, origin) = opened();
        let sphere = generator.sphere();
        let site = &cells.sites[0];
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
