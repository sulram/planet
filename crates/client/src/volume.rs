//! The ground as a volume, where the quadtree runs out of levels.
//!
//! A heightfield has one ground per column, so it can carry a cliff but never
//! a roof: no cave, no arch, no overhang. At the deepest level of the quadtree
//! a patch is exactly `2 x 2` chunks across and one block a cell, so the same
//! square of ground is meshed from the density instead, by surface nets.
//!
//! That the handover lands on a level boundary is the whole point of doing it
//! here. The streamer already builds patches nearest and coarsest first, the
//! neighbour one level up already carries a skirt, and the two agree on where
//! the ground is because `density_m` crosses zero at exactly the height
//! `sample_at` reports. Nothing new streams and nothing has to be suppressed.

use glam::DVec3;
use scene::TerrainVertex;
use topology::{BLOCK_M, Sector, SurfacePoint};
use voxel::{CHUNK_SIDE, Material as MaterialId};
use worldgen::{Column, Generator, Material};

/// Metres of ground kept under the lowest ground of a patch, and of air over
/// the highest: enough for a cave mouth and the lip over it.
///
/// How far down the volume reaches is what a frame can afford and nothing
/// else. Measured, on the descent the bench times: two chunks of stack cost
/// 10.7 ms of a 12 ms frame, three cost 12.6 and blow it. Terrain is still
/// built on the frame's own thread, and the job queue M1.5 already asks for
/// is what lifts this, not a cleverer mesher. Until then a cave is something
/// seen from its mouth rather than followed down, which is also all that
/// collision can offer it.
const BELOW_M: f64 = 8.0;
const ABOVE_M: f64 = 3.0;
/// Chunks a patch may stack. A cliff inside one patch is what this bounds.
const TALL: i64 = 2;

/// Blocks across a patch at the deepest level, which is `CHUNK_SIDE` twice.
pub const PATCH_BLOCKS: i64 = CHUNK_SIDE as i64 * 2;
/// Columns across a patch, with the border its chunks share.
const COLUMNS: i64 = PATCH_BLOCKS + 2;

/// Everything about the columns of one patch, worked out once. A chunk asks
/// for a column many times going up it, and the ground under that column does
/// not change with height.
struct Ground<'a> {
    columns: &'a [Column],
    /// Block coordinates of this chunk's low corner.
    origin: [i64; 3],
    /// Block coordinates of the patch's low corner, one in from the border.
    patch: [i64; 2],
}

impl Ground<'_> {
    fn column(&self, at: [i32; 3]) -> &Column {
        let u = self.origin[0] + i64::from(at[0]) - self.patch[0] + 1;
        let v = self.origin[1] + i64::from(at[1]) - self.patch[1] + 1;
        &self.columns[(u + COLUMNS * v) as usize]
    }

    fn height_m(&self, at: [i32; 3]) -> f64 {
        (self.origin[2] + i64::from(at[2])) as f64 * BLOCK_M
    }
}

impl voxel::Ground for Ground<'_> {
    fn density(&self, at: [i32; 3]) -> f32 {
        self.column(at).density_m(self.height_m(at)) as f32
    }

    fn material(&self, at: [i32; 3]) -> MaterialId {
        self.column(at).ground().material as MaterialId
    }
}

/// The surface of one patch, and the indices that join it. Empty when the
/// ground turns out to pass through none of the chunks, which cannot happen
/// on a patch the streamer asked for but costs nothing to say.
pub fn build(
    generator: &Generator,
    sector: Sector,
    patch: [i64; 2],
    radius_m: f64,
    origin: DVec3,
    color: impl Fn(Material) -> [u8; 4],
) -> (Vec<TerrainVertex>, Vec<u32>) {
    // The columns of the patch and of the border its chunks reach into.
    let mut columns = Vec::with_capacity((COLUMNS * COLUMNS) as usize);
    for v in 0..COLUMNS {
        for u in 0..COLUMNS {
            let point =
                SurfacePoint::new(sector, (patch[0] + u - 1) as f64, (patch[1] + v - 1) as f64);
            // The same footprint the heightfield patch of this level would
            // have used. A cell is one block, so asking for more detail than
            // that would draw a different surface from the level above and
            // stand the difference up as a wall at the LOD ring.
            columns.push(generator.column(point.direction(), BLOCK_M));
        }
    }

    // How far up and down the ground of this patch reaches, in chunks.
    let (mut lowest, mut highest) = (f64::MAX, f64::MIN);
    for column in &columns {
        let ground_m = column.ground().height_m;
        lowest = lowest.min(ground_m);
        highest = highest.max(ground_m);
    }
    let chunk_m = CHUNK_SIDE as f64 * BLOCK_M;
    let top = libm::floor((highest + ABOVE_M) / chunk_m) as i64;
    let floor = libm::floor((lowest - BELOW_M) / chunk_m) as i64;
    let bottom = floor.max(top - TALL + 1);

    let mut vertices: Vec<TerrainVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for h in bottom..=top {
        for v in 0..2 {
            for u in 0..2 {
                let chunk = [
                    patch[0] + u * CHUNK_SIDE as i64,
                    patch[1] + v * CHUNK_SIDE as i64,
                    h * CHUNK_SIDE as i64,
                ];
                if !worth_meshing(&columns, chunk, patch) {
                    continue;
                }
                let mesh = voxel::mesh(&Ground {
                    columns: &columns,
                    origin: chunk,
                    patch,
                });
                if mesh.is_empty() {
                    continue;
                }
                let first = vertices.len() as u32;
                indices.extend(mesh.indices.iter().map(|i| i + first));
                vertices.extend(mesh.vertices.iter().map(|vertex| {
                    place(
                        &vertex.position,
                        &vertex.normal,
                        vertex.material,
                        chunk,
                        sector,
                        radius_m,
                        origin,
                        &color,
                    )
                }));
            }
        }
    }
    (vertices, indices)
}

/// Whether the ground can pass through a chunk at all.
///
/// The columns are already worked out, so this is free, and it pays for
/// itself many times over: a stack of chunks under a hillside is mostly rock
/// with nothing in it, and each one asked outright costs 5,832 samples to
/// learn that. Chunks wholly over the ground are air and chunks wholly under
/// it are rock, unless a column has a cave, which may be anywhere in it.
fn worth_meshing(columns: &[Column], chunk: [i64; 3], patch: [i64; 2]) -> bool {
    let low_m = chunk[2] as f64 * BLOCK_M;
    let high_m = (chunk[2] + CHUNK_SIDE as i64) as f64 * BLOCK_M;
    let (mut all_under, mut all_over) = (true, true);
    for v in -1..=CHUNK_SIDE as i64 {
        for u in -1..=CHUNK_SIDE as i64 {
            let at = (chunk[0] + u - patch[0] + 1) + COLUMNS * (chunk[1] + v - patch[1] + 1);
            let column = columns[at as usize];
            let ground_m = column.ground().height_m;
            // A cave is a hole in rock that looks solid from its ground alone.
            all_under &= ground_m >= high_m && column.solid();
            all_over &= ground_m <= low_m;
            if !all_under && !all_over {
                return true;
            }
        }
    }
    false
}

/// One vertex from the cells it was found in to the world it is drawn in.
#[allow(clippy::too_many_arguments)]
fn place(
    position: &[f32; 3],
    normal: &[f32; 3],
    material: MaterialId,
    chunk: [i64; 3],
    sector: Sector,
    radius_m: f64,
    origin: DVec3,
    color: &impl Fn(Material) -> [u8; 4],
) -> TerrainVertex {
    let block = |axis: usize| chunk[axis] as f64 + f64::from(position[axis]);
    // One frame answers both questions: `up` is the direction, and asking for
    // it twice would pay for the tangent warp twice on every vertex.
    let tangents = SurfacePoint::new(sector, block(0), block(1)).tangents();
    let direction = DVec3::from(tangents.up);
    let height_m = block(2) * BLOCK_M;
    let world = direction * (radius_m + height_m);

    // A cell is not a cube in the world: the tangent warp makes a block wider
    // at a sector's middle than at its corner, and up is up. A normal turns by
    // the inverse of that, which for a frame this near square is each axis
    // over its own length.
    let (du, dv) = (DVec3::from(tangents.du), DVec3::from(tangents.dv));
    let scale = radius_m + height_m;
    let along = |axis: DVec3, n: f32, size: f64| axis.normalize() * (f64::from(n) / size);
    let world_normal = along(du, normal[0], du.length() * scale)
        + along(dv, normal[1], dv.length() * scale)
        + direction * (f64::from(normal[2]) / BLOCK_M);

    TerrainVertex {
        position: (world - origin).as_vec3().to_array(),
        normal: world_normal.normalize_or(DVec3::Y).as_vec3().to_array(),
        color: color(Material::from_id(material).unwrap_or(Material::Rock)),
    }
}
