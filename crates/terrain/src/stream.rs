//! What a renderer should be holding, and the changes that get it there.
//!
//! One rule, not a window: a chunk is wanted when its centre is within reach
//! of the eye and inside the build band of the column it stands on. Nothing
//! here is capped against a frame budget, because a cap chosen that way is a
//! constant tuned against a picture (DECISIONS 48).
//!
//! Reach is short while the far field is missing: at 8 m a chunk, seeing a
//! kilometre is a pyramid of reduced levels and not more chunks. A body small
//! enough to fit in the near field is therefore whole, which is what makes a
//! small world worth proving on.

use std::collections::{HashMap, HashSet};

use glam::DVec3;
use scene::{PatchId, TerrainChange};
use topology::{BLOCK_M, Column, QuadSphere, SurfacePoint};
use voxel::CHUNK_SIDE;
use worldgen::Generator;

use crate::address::{ChunkAddr, band_chunks, chunk_grid};
use crate::generate::FOOTPRINT_M;
use crate::mesh::mesh;
use crate::read::Chunks;

/// Metres along one edge of a chunk.
pub const CHUNK_M: f64 = CHUNK_SIDE as f64 * BLOCK_M;

/// Chunks a frame may generate.
///
/// Measured by `bun run bench`, which is the referee, not this machine: in
/// WASM a chunk costs several times what it costs in a native release build,
/// so four of them is a frame of about 4 ms against a budget of 12, and the
/// worst frame of a ten second descent is 3.75 ms with none over.
///
/// The number only means anything because the budget can stop in the middle of
/// a neighbourhood. Before it could, one chunk at the frontier pulled in 27
/// and cost 20 ms whatever this was set to.
pub const BUDGET: usize = 4;

/// How far the near field reaches, in metres. Everything inside it is full
/// resolution voxels; there is nothing outside it yet.
pub const REACH_M: f64 = 56.0;

/// The ground around one body, streamed.
pub struct Terrain {
    sphere: QuadSphere,
    chunks: Chunks,
    /// Chunks a renderer holds, and what it calls them.
    drawn: HashMap<ChunkAddr, PatchId>,
    /// Chunks that were meshed and had no surface in them. Remembered, or
    /// every frame would build the solid rock under the ground again.
    blank: HashSet<ChunkAddr>,
    /// What is wanted and not built yet, nearest last: a frame pops from the
    /// end, so what appears first is what you are standing next to.
    queue: Vec<ChunkAddr>,
    /// The chunk the eye was in when the queue was last worked out. The wanted
    /// set only changes when the eye leaves it.
    standing_in: Option<ChunkAddr>,
    changes: Vec<TerrainChange>,
    next_id: u64,
    reach_m: f64,
}

impl Terrain {
    pub fn new(sphere: QuadSphere) -> Terrain {
        Terrain {
            sphere,
            chunks: Chunks::new(),
            drawn: HashMap::new(),
            blank: HashSet::new(),
            queue: Vec::new(),
            standing_in: None,
            changes: Vec::new(),
            next_id: 1,
            reach_m: REACH_M,
        }
    }

    pub fn sphere(&self) -> QuadSphere {
        self.sphere
    }

    /// How coarse the drawn ground is, metres between samples. The near field
    /// is full resolution, so collision and what you see agree exactly.
    pub fn drawn_footprint_m(&self) -> f64 {
        FOOTPRINT_M
    }

    /// Everything a renderer holds, in no particular order.
    pub fn drawn(&self) -> Vec<PatchId> {
        self.drawn.values().copied().collect()
    }

    pub fn held_chunks(&self) -> usize {
        self.chunks.len()
    }

    pub fn drain_changes(&mut self) -> Vec<TerrainChange> {
        core::mem::take(&mut self.changes)
    }

    /// Drops everything, so a new recipe starts from nothing.
    pub fn clear(&mut self) {
        for id in self.drawn.values() {
            self.changes.push(TerrainChange::Remove(*id));
        }
        self.drawn.clear();
        self.blank.clear();
        self.queue.clear();
        self.standing_in = None;
        self.chunks = Chunks::new();
    }

    /// Brings the held set in line with where the eye is, building at most
    /// `budget` chunks, and answers with what to draw. `eye` is metres from
    /// this body's centre.
    ///
    /// The wanted set is worked out only when the eye leaves the chunk it was
    /// in, because walking within one chunk cannot change it. What is left is
    /// a queue, drained a few chunks a frame: a chunk costs what it costs, and
    /// a frame that builds every chunk it wants is a frame that stalls.
    pub fn update(&mut self, generator: &Generator, eye: DVec3, budget: usize) -> Vec<PatchId> {
        let here = self.eye_chunk(eye);
        if self.standing_in != Some(here) {
            self.standing_in = Some(here);
            self.resettle(generator, eye);
        }
        self.build(generator, budget);
        self.drawn()
    }

    /// The same, with every wanted chunk built before it answers. For headless
    /// renders, where there is no next frame to finish the job.
    pub fn settle(&mut self, generator: &Generator, eye: DVec3) -> Vec<PatchId> {
        self.standing_in = Some(self.eye_chunk(eye));
        self.resettle(generator, eye);
        self.build(generator, usize::MAX);
        self.drawn()
    }

    /// Which chunk the eye is standing in, whatever is under it.
    fn eye_chunk(&self, eye: DVec3) -> ChunkAddr {
        let grid = chunk_grid(self.sphere);
        let point = grid.surface_point([eye.x, eye.y, eye.z]);
        let above_m = eye.length() - self.sphere.radius_m();
        let h = (above_m / CHUNK_M).floor().clamp(f64::from(i16::MIN), f64::from(i16::MAX));
        ChunkAddr::new(grid.column_of(point), h as i16)
    }

    /// Works out what is wanted, drops what is not, and queues the rest.
    fn resettle(&mut self, generator: &Generator, eye: DVec3) {
        let wanted = self.wanted(generator, eye);

        // Gone first, so a move never holds both sets at once.
        let stale: Vec<ChunkAddr> = self
            .drawn
            .keys()
            .filter(|addr| !wanted.contains(*addr))
            .copied()
            .collect();
        for addr in stale {
            if let Some(id) = self.drawn.remove(&addr) {
                self.changes.push(TerrainChange::Remove(id));
            }
        }
        self.blank.retain(|addr| wanted.contains(addr));

        // The cache holds the near field and the border a mesh reads, and
        // nothing else: a chunk just outside the reach is about to be a
        // neighbour again, so dropping it the moment it stops being drawn
        // would generate it twice for one step.
        self.chunks
            .retain_near(eye, self.reach_m + CHUNK_M * 2.0);

        // Distance once per chunk, not once per comparison: the key is worked
        // out here and the sort only reads it.
        let sphere = self.sphere;
        let mut by_distance: Vec<(f64, ChunkAddr)> = wanted
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

    /// Works the queue until `budget` chunks have been generated.
    ///
    /// The budget counts chunks made, not meshes finished, because a mesh at
    /// the frontier pulls in up to 27 of them: it reads one cell past itself
    /// on every side, and those cells belong to neighbours. Counting meshes
    /// would let one frame do twenty times the work of another.
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

    /// How much is still waiting to be built.
    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Every chunk wanted at `eye`: inside the reach, inside the band.
    fn wanted(&self, generator: &Generator, eye: DVec3) -> HashSet<ChunkAddr> {
        let sphere = self.sphere;
        let grid = chunk_grid(sphere);
        let blocks = sphere.blocks();
        let mut wanted = HashSet::new();

        // Out of reach of the whole body: nothing is wanted, and nothing is
        // worth asking the generator. Flying to another body must not cost a
        // scan of this one every frame.
        let above_m = eye.length() - sphere.radius_m();
        let under_eye = blocks.surface_point([eye.x, eye.y, eye.z]);
        let ground_under_m = generator
            .sample_at(blocks.direction(under_eye), FOOTPRINT_M)
            .height_m;
        // Measured from the ground, because that is what the band hangs off.
        if above_m - ground_under_m > self.reach_m + sphere.band_m() + CHUNK_M {
            return wanted;
        }

        let here = grid.surface_point([eye.x, eye.y, eye.z]);
        let span = (self.reach_m / CHUNK_M).ceil() as i32;
        // A body smaller than the reach is wholly in the near field, and a
        // square wider than the grid would ask to step several faces at once,
        // which is not a step. Take every column instead.
        let whole = 2 * span + 1 >= grid.side() as i32;
        let band = i32::from(band_chunks(sphere));
        let eye_h = (above_m / CHUNK_M).floor() as i32;

        let radius_m = sphere.radius_m();
        let slack = CHUNK_M * 0.87;
        let reach = self.reach_m + slack;
        for column in columns(grid, here, span, whole) {
            // One direction per chunk column. Every chunk above it sits on
            // this ray, so its position is arithmetic from here: working the
            // sphere out per chunk instead meant thousands of tangents in the
            // frame that first comes into range (measured: it was the spike).
            let low = Column::new(
                column.sector,
                column.u << voxel::CHUNK_BITS,
                column.v << voxel::CHUNK_BITS,
            );
            let half = CHUNK_SIDE as f64 / 2.0;
            let middle = SurfacePoint::new(
                low.sector,
                f64::from(low.u) + half,
                f64::from(low.v) + half,
            );
            let ray = DVec3::from(blocks.direction(blocks.wrapped(middle)));

            let under = generator.column(ray.to_array(), FOOTPRINT_M);
            let ground_h = (under.ground().height_m / CHUNK_M).floor() as i32;

            // The band is what may be edited, not what has to be drawn. Where
            // the generator says nothing under this column is hollow, only the
            // chunks the surface itself runs through can hold one; where there
            // are caves, the whole band can.
            let deep = if under.solid() { 1 } else { band };
            let low_h = ground_h.saturating_sub(deep).max(eye_h.saturating_sub(span));
            let high_h = ground_h.saturating_add(deep).min(eye_h.saturating_add(span));
            for h in low_h..=high_h {
                let Ok(h) = i16::try_from(h) else { continue };
                let centre_m = (f64::from(i32::from(h) * CHUNK_SIDE as i32) + half) * BLOCK_M;
                if (ray * (radius_m + centre_m)).distance(eye) <= reach {
                    wanted.insert(ChunkAddr::new(column, h));
                }
            }
        }
        wanted
    }

}

/// The chunk columns to consider: every one of them on a body that fits in
/// the near field, and the square around the eye otherwise.
fn columns(
    grid: topology::Grid,
    here: SurfacePoint,
    span: i32,
    whole: bool,
) -> Vec<Column> {
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
    let half = CHUNK_SIDE as f64 / 2.0;
    let low = addr.low_column();
    let point = SurfacePoint::new(
        low.sector,
        f64::from(low.u) + half,
        f64::from(low.v) + half,
    );
    let height_m = (f64::from(addr.low_h()) + half) * BLOCK_M;
    let at = sphere.position(sphere.blocks().wrapped(point), height_m);
    DVec3::new(at[0], at[1], at[2])
}
