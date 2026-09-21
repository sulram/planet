//! What a renderer should be holding, and the changes that get it there.
//!
//! A pyramid of levels, the near one fine and the far ones coarse. A level's
//! chunks are wanted in a shell: close enough that the level is worth drawing,
//! far enough that the level under it does not already cover the ground. The
//! coarsest level has no outer edge, so a body is drawn whole from any
//! distance and never disappears.
//!
//! Each level is a square of fixed width around the eye, so the work per level
//! does not grow with the body, and a level is only reworked when the eye
//! leaves the chunk it was in *at that level*: the coarse ones almost never
//! move (DECISIONS 52).

use std::collections::{HashMap, HashSet};

use glam::DVec3;
use scene::{PatchId, TerrainChange};
use topology::{Column, Grid, QuadSphere, SurfacePoint};
use voxel::{CHUNK_BITS, CHUNK_SIDE};
use worldgen::Generator;

use crate::address::{ChunkAddr, Level, band_chunks, chunk_grid, coarsest};
use crate::mesh::mesh;
use crate::read::Chunks;

/// Metres along one edge of a level 0 chunk.
pub const CHUNK_M: f64 = CHUNK_SIDE as f64 * topology::BLOCK_M;

/// How many of its own widths from the eye a level stays worth drawing.
///
/// This is the only knob the pyramid has, and it sets both the detail and the
/// cost: every level is a square `2 * DETAIL + 1` chunks across, so raising it
/// raises the work at *every* level at once.
pub const DETAIL: f64 = 7.0;

/// Chunks a frame may generate.
///
/// Measured by `bun run bench`, which is the referee, not this machine: in
/// WASM a chunk costs several times what it costs in a native release build.
///
/// The number only means anything because the budget can stop in the middle of
/// a neighbourhood. Before it could, one chunk at the frontier pulled in 27
/// and cost 20 ms whatever this was set to (DECISIONS 51).
pub const BUDGET: usize = 4;

/// How far a level reaches, metres.
fn reach_m(level: Level) -> f64 {
    DETAIL * CHUNK_M * f64::from(1u32 << level)
}

/// What one level of the pyramid is holding.
#[derive(Default)]
struct LevelState {
    /// The chunk the eye was in when this level was last worked out.
    standing_in: Option<ChunkAddr>,
    wanted: HashSet<ChunkAddr>,
}

/// The ground around one body, streamed.
pub struct Terrain {
    sphere: QuadSphere,
    chunks: Chunks,
    /// Chunks a renderer holds, and what it calls them.
    drawn: HashMap<ChunkAddr, PatchId>,
    /// Chunks that were meshed and had no surface in them. Remembered, or
    /// every frame would build the solid rock under the ground again.
    blank: HashSet<ChunkAddr>,
    /// Chunks the pyramid no longer wants, still drawn because what replaces
    /// them is not ready. Nothing is ever taken away before its replacement
    /// is up: that is what a hole at a level boundary is made of.
    retiring: HashMap<ChunkAddr, PatchId>,
    /// What is wanted and not built yet, nearest last: a frame pops from the
    /// end, so what appears first is what you are standing next to.
    queue: Vec<ChunkAddr>,
    levels: Vec<LevelState>,
    changes: Vec<TerrainChange>,
    next_id: u64,
}

impl Terrain {
    pub fn new(sphere: QuadSphere) -> Terrain {
        let levels = (0..=coarsest(sphere)).map(|_| LevelState::default()).collect();
        Terrain {
            sphere,
            chunks: Chunks::new(),
            drawn: HashMap::new(),
            retiring: HashMap::new(),
            blank: HashSet::new(),
            queue: Vec::new(),
            levels,
            changes: Vec::new(),
            next_id: 1,
        }
    }

    pub fn sphere(&self) -> QuadSphere {
        self.sphere
    }

    /// How coarse the ground under the eye is drawn, metres between samples.
    /// The near field is full resolution, so collision and what you see agree.
    pub fn drawn_footprint_m(&self) -> f64 {
        topology::BLOCK_M
    }

    /// Everything a renderer holds, in no particular order: what the pyramid
    /// wants, and what it has not finished replacing.
    pub fn drawn(&self) -> Vec<PatchId> {
        self.drawn
            .values()
            .chain(self.retiring.values())
            .copied()
            .collect()
    }

    /// What casts a shadow: the near levels of what the pyramid wants.
    ///
    /// Not what is retiring, which is up only so the ground has no hole in it
    /// and would double every shadow through the handover. And not ground
    /// coarser than [`scene::SHADOW_CASTER_CELL_M`], whose triangles are wider
    /// than a shadow texel and turn black on themselves.
    pub fn casters(&self) -> Vec<PatchId> {
        self.drawn
            .iter()
            .filter(|(addr, _)| addr.cell_m() <= scene::SHADOW_CASTER_CELL_M)
            .map(|(_, id)| *id)
            .collect()
    }

    /// How many chunks are still up only because their replacement is not.
    pub fn retiring(&self) -> usize {
        self.retiring.len()
    }

    /// Every chunk a renderer holds, with the address behind it. For tests
    /// that have to know what a patch id stands for.
    pub fn named(&self) -> Vec<(PatchId, ChunkAddr)> {
        self.drawn
            .iter()
            .chain(self.retiring.iter())
            .map(|(addr, id)| (*id, *addr))
            .collect()
    }

    pub fn held_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// How much is still waiting to be built.
    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    pub fn drain_changes(&mut self) -> Vec<TerrainChange> {
        core::mem::take(&mut self.changes)
    }

    /// Drops everything, so a new recipe starts from nothing.
    pub fn clear(&mut self) {
        for id in self.drawn.values().chain(self.retiring.values()) {
            self.changes.push(TerrainChange::Remove(*id));
        }
        self.drawn.clear();
        self.retiring.clear();
        self.blank.clear();
        self.queue.clear();
        for level in &mut self.levels {
            *level = LevelState::default();
        }
        self.chunks = Chunks::new();
    }

    /// Brings the held set in line with where the eye is, building at most
    /// `budget` chunks, and answers with what to draw. `eye` is metres from
    /// this body's centre.
    pub fn update(&mut self, generator: &Generator, eye: DVec3, budget: usize) -> Vec<PatchId> {
        if self.resettle(generator, eye) {
            self.requeue(eye);
        }
        self.build(generator, budget);
        self.retire();
        self.drawn()
    }

    /// The same, with every wanted chunk built before it answers. For headless
    /// renders, where there is no next frame to finish the job.
    pub fn settle(&mut self, generator: &Generator, eye: DVec3) -> Vec<PatchId> {
        for level in &mut self.levels {
            level.standing_in = None;
        }
        self.resettle(generator, eye);
        self.requeue(eye);
        self.build(generator, usize::MAX);
        self.retire();
        self.drawn()
    }

    /// Reworks the levels the eye has moved out of. Answers whether anything
    /// changed.
    fn resettle(&mut self, generator: &Generator, eye: DVec3) -> bool {
        let mut moved = false;
        // Finest first: a level's inner edge is cut out of the level under it,
        // so when that one moves this one has to follow, however still the eye
        // is at this grain.
        let mut finer_moved = false;
        for level in 0..self.levels.len() as Level {
            let here = self.eye_chunk(level, eye);
            let own = here.is_some() && self.levels[level as usize].standing_in != here;
            if !own && !finer_moved {
                finer_moved = false;
                continue;
            }
            if let Some(here) = here {
                self.levels[level as usize].standing_in = Some(here);
            }
            let wanted = self.wanted(generator, eye, level);
            self.levels[level as usize].wanted = wanted;
            finer_moved = true;
            moved = true;
        }
        moved
    }

    /// Which chunk of `level` the eye is standing in.
    fn eye_chunk(&self, level: Level, eye: DVec3) -> Option<ChunkAddr> {
        let grid = chunk_grid(self.sphere, level)?;
        let point = grid.surface_point([eye.x, eye.y, eye.z]);
        let span = CHUNK_M * f64::from(1u32 << level);
        let above_m = eye.length() - self.sphere.radius_m();
        let h = (above_m / span)
            .floor()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX));
        Some(ChunkAddr::new(level, grid.column_of(point), h as i16))
    }

    /// Everything the pyramid wants, dropping what it no longer does and
    /// queueing what is missing.
    fn requeue(&mut self, eye: DVec3) {
        let union: HashSet<ChunkAddr> = self
            .levels
            .iter()
            .flat_map(|level| level.wanted.iter().copied())
            .collect();

        // What the pyramid stopped wanting stays up, and joins the set that
        // is waiting to be replaced. Taking it away here is what opened a hole
        // every time a level boundary moved: the chunk that covers the same
        // ground at the other level is still in the queue, frames away.
        let stale: Vec<ChunkAddr> = self
            .drawn
            .keys()
            .filter(|addr| !union.contains(*addr))
            .copied()
            .collect();
        for addr in stale {
            if let Some(id) = self.drawn.remove(&addr) {
                self.retiring.insert(addr, id);
            }
        }
        // Something wanted again is no longer retiring.
        let returned: Vec<ChunkAddr> = self
            .retiring
            .keys()
            .filter(|addr| union.contains(*addr))
            .copied()
            .collect();
        for addr in returned {
            if let Some(id) = self.retiring.remove(&addr) {
                self.drawn.insert(addr, id);
            }
        }
        self.blank.retain(|addr| union.contains(addr));

        // The cache holds each level's shell and the border a mesh reads. A
        // chunk just outside is about to be a neighbour again, so dropping it
        // the moment it stops being drawn would generate it twice for a step.
        self.chunks.retain_near(eye, |addr| {
            reach_m(addr.level) + 2.0 * addr.span_m()
        });

        // Distance once per chunk, not once per comparison.
        let sphere = self.sphere;
        let mut by_distance: Vec<(f64, ChunkAddr)> = union
            .into_iter()
            .filter(|addr| !self.drawn.contains_key(addr) && !self.blank.contains(addr))
            .map(|addr| {
                let centre = self
                    .chunks
                    .centre(&addr)
                    .unwrap_or_else(|| chunk_centre(sphere, addr));
                (centre.distance_squared(eye), addr)
            })
            .collect();
        // Furthest first, so popping from the end builds what is nearest.
        by_distance.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(core::cmp::Ordering::Equal));
        self.queue = by_distance.into_iter().map(|(_, addr)| addr).collect();
    }

    /// The chunks one level wants: its own shell around the eye.
    fn wanted(&self, generator: &Generator, eye: DVec3, level: Level) -> HashSet<ChunkAddr> {
        let sphere = self.sphere;
        let mut wanted = HashSet::new();
        let (Some(grid), Some(cells)) = (
            chunk_grid(sphere, level),
            crate::address::cell_grid(sphere, level),
        ) else {
            return wanted;
        };

        let outermost = level == coarsest(sphere);
        let span_m = CHUNK_M * f64::from(1u32 << level);
        let far = reach_m(level) + span_m * 0.87;

        let here = grid.surface_point([eye.x, eye.y, eye.z]);
        let square = DETAIL.ceil() as i32;
        let whole = 2 * square + 1 >= grid.side() as i32;
        let band = i32::from(band_chunks(sphere, level));
        let radius_m = sphere.radius_m();
        let cell_m = span_m / CHUNK_SIDE as f64;
        let half = CHUNK_SIDE as f64 / 2.0;

        for column in columns(grid, here, square, whole) {
            // One direction per chunk column. Every chunk above it sits on
            // this ray, so its position is arithmetic from here.
            let low = Column::new(
                column.sector,
                column.u << CHUNK_BITS,
                column.v << CHUNK_BITS,
            );
            let middle = SurfacePoint::new(
                low.sector,
                f64::from(low.u) + half,
                f64::from(low.v) + half,
            );
            let ray = DVec3::from(cells.direction(cells.wrapped(middle)));

            let under = generator.column(ray.to_array(), cell_m);
            let ground_h = (under.ground().height_m / span_m).floor() as i32;

            // The band is what may be edited, not what has to be drawn. Where
            // the generator says nothing under this column is hollow, only the
            // chunks the surface runs through can hold one.
            let deep = if under.solid() { 1 } else { band };
            for h in ground_h.saturating_sub(deep)..=ground_h.saturating_add(deep) {
                let Ok(h) = i16::try_from(h) else { continue };
                let centre_m = (f64::from(i32::from(h) * CHUNK_SIDE as i32) + half) * cell_m;
                let distance = (ray * (radius_m + centre_m)).distance(eye);
                if distance > far && !outermost {
                    continue;
                }
                let addr = ChunkAddr::new(level, column, h);
                // Exactly one level draws any piece of ground. A chunk whose
                // eight children are all wanted one level finer is already
                // covered, so it is cut out rather than drawn on top: two
                // surfaces in one place fight for the depth buffer and cast
                // two shadows (DECISIONS 54).
                if self.covered_by_finer(addr) {
                    continue;
                }
                wanted.insert(addr);
            }
        }
        wanted
    }

    /// Works the queue until `budget` chunks have been generated.
    ///
    /// The budget counts chunks made, not meshes finished, because a mesh at
    /// the frontier pulls in up to 27 of them: it reads one cell past itself
    /// on every side, and those cells belong to neighbours (DECISIONS 51).
    fn build(&mut self, generator: &Generator, budget: usize) {
        let mut made = 0;
        while made < budget {
            let Some(&addr) = self.queue.last() else { break };
            if self.drawn.contains_key(&addr) || self.blank.contains(&addr) {
                self.queue.pop();
                continue;
            }
            // Only as much of the neighbourhood as the budget allows. If it is
            // not whole yet, this chunk stays at the front of the queue and
            // the next frame carries on where this one stopped.
            let warmth = self.chunks.warm(generator, self.sphere, addr, budget - made);
            made += warmth.made;
            if !warmth.whole {
                break;
            }
            self.queue.pop();
            let Some(mesh) = mesh(&self.chunks, self.sphere, addr) else {
                // Solid rock, or open sky. Nothing to draw, and nothing to
                // come back to until an edit puts a surface in it.
                self.blank.insert(addr);
                continue;
            };
            let id = PatchId(self.next_id);
            self.next_id += 1;
            self.drawn.insert(addr, id);
            self.changes.push(TerrainChange::Add(id, mesh));
        }
    }
}

impl Terrain {
    /// Takes down what the pyramid stopped wanting, but only once the ground
    /// it covered is drawn at the level that replaced it.
    ///
    /// A stale chunk is replaced by its parent when the eye moves away and by
    /// its eight children when it moves closer, because the shells that meet
    /// are always one level apart. When neither is coming - the band moved, or
    /// the world was left behind - the queue running dry is what says so.
    fn retire(&mut self) {
        if self.retiring.is_empty() {
            return;
        }
        let settled = self.queue.is_empty();
        let ready: Vec<ChunkAddr> = self
            .retiring
            .keys()
            .filter(|addr| settled || self.replaced(**addr))
            .copied()
            .collect();
        for addr in ready {
            if let Some(id) = self.retiring.remove(&addr) {
                self.changes.push(TerrainChange::Remove(id));
            }
        }
    }

    /// Whether every piece of ground this chunk covers is wanted one level
    /// finer, so drawing it too would double the surface.
    fn covered_by_finer(&self, addr: ChunkAddr) -> bool {
        let Some(finer) = addr.level.checked_sub(1) else {
            return false;
        };
        let Some(children) = self.children(addr) else {
            return false;
        };
        let below = &self.levels[finer as usize].wanted;
        children.iter().all(|child| below.contains(child))
    }

    /// Whether the ground a chunk covers is drawn at another level now.
    fn replaced(&self, addr: ChunkAddr) -> bool {
        if let Some(parent) = self.parent(addr) {
            if self.settled(parent) {
                return true;
            }
        }
        match self.children(addr) {
            Some(children) => children.iter().all(|child| self.settled(*child)),
            None => false,
        }
    }

    /// Built one way or the other: drawn, or known to hold no surface.
    fn settled(&self, addr: ChunkAddr) -> bool {
        self.drawn.contains_key(&addr) || self.blank.contains(&addr)
    }

    /// The chunk one level coarser that covers the same ground.
    fn parent(&self, addr: ChunkAddr) -> Option<ChunkAddr> {
        let level = addr.level + 1;
        chunk_grid(self.sphere, level)?;
        Some(ChunkAddr::new(
            level,
            Column::new(addr.column.sector, addr.column.u >> 1, addr.column.v >> 1),
            addr.h.div_euclid(2),
        ))
    }

    /// The eight chunks one level finer that cover the same ground.
    fn children(&self, addr: ChunkAddr) -> Option<[ChunkAddr; 8]> {
        let level = addr.level.checked_sub(1)?;
        let (u, v) = (addr.column.u * 2, addr.column.v * 2);
        let h = addr.h.checked_mul(2)?;
        let mut all = [addr; 8];
        for (i, slot) in all.iter_mut().enumerate() {
            *slot = ChunkAddr::new(
                level,
                Column::new(
                    addr.column.sector,
                    u + (i & 1) as u16,
                    v + ((i >> 1) & 1) as u16,
                ),
                h + ((i >> 2) & 1) as i16,
            );
        }
        Some(all)
    }
}

/// The chunk columns to consider: every one of them on a grid that fits in the
/// square, and the square around the eye otherwise.
fn columns(grid: Grid, here: SurfacePoint, span: i32, whole: bool) -> Vec<Column> {
    if whole {
        let last = grid.max_coord();
        return topology::Sector::ALL
            .into_iter()
            .flat_map(|sector| {
                (0..=last).flat_map(move |v| (0..=last).map(move |u| Column::new(sector, u, v)))
            })
            .collect();
    }
    let mut all = Vec::with_capacity(((2 * span + 1) * (2 * span + 1)) as usize);
    for dv in -span..=span {
        for du in -span..=span {
            let point = SurfacePoint::new(
                here.sector,
                here.u.floor() + f64::from(du) + 0.5,
                here.v.floor() + f64::from(dv) + 0.5,
            );
            all.push(grid.column_of(grid.wrapped(point)));
        }
    }
    all
}

/// Where a chunk's middle sits, metres from the body's centre.
pub(crate) fn chunk_centre(sphere: QuadSphere, addr: ChunkAddr) -> DVec3 {
    let cells = crate::address::cell_grid(sphere, addr.level).expect("a level this body has");
    let half = CHUNK_SIDE as f64 / 2.0;
    let low = addr.low_column();
    let point = SurfacePoint::new(
        low.sector,
        f64::from(low.u) + half,
        f64::from(low.v) + half,
    );
    let height_m = (f64::from(addr.low_h()) + half) * addr.cell_m();
    let at = cells.direction(cells.wrapped(point));
    DVec3::new(at[0], at[1], at[2]) * (sphere.radius_m() + height_m)
}
