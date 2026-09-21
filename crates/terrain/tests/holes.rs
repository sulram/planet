//! No hole, ever: the ground under the eye is drawn at some level at every
//! moment of a descent, including the frames where one level hands over to
//! the next.
//!
//! This is the invariant a screenshot cannot check. A level boundary moves
//! whenever the eye does, and taking the old chunk down before the new one is
//! up leaves nothing there for as long as the queue takes to reach it.

use std::collections::HashSet;

use glam::DVec3;
use scene::{PatchId, TerrainChange};
use terrain::{BUDGET, ChunkAddr, Terrain, chunk_grid, coarsest};
use topology::{QuadSphere, SurfacePoint};
use voxel::CHUNK_SIDE;
use worldgen::{Generator, Recipe};

fn body(bits: u32) -> (Generator, QuadSphere) {
    let mut recipe = Recipe::new(1);
    recipe.sector_bits = bits;
    let generator = Generator::new(recipe).expect("a generator");
    let sphere = generator.sphere();
    (generator, sphere)
}

/// The chunk of `level` that holds the ground under a direction.
fn ground_chunk(
    generator: &Generator,
    sphere: QuadSphere,
    direction: DVec3,
    level: u32,
) -> Option<ChunkAddr> {
    let grid = chunk_grid(sphere, level)?;
    let span_m = f64::from(CHUNK_SIDE as u32) * topology::BLOCK_M * f64::from(1u32 << level);
    let cell_m = span_m / f64::from(CHUNK_SIDE as u32);
    let ground_m = generator
        .sample_at(direction.to_array(), cell_m)
        .height_m;
    let h = (ground_m / span_m).floor();
    let h = i16::try_from(h as i64).ok()?;
    Some(ChunkAddr::new(
        level,
        grid.column_of(grid.surface_point(direction.to_array())),
        h,
    ))
}

/// Whether the ground under a direction is drawn at any level at all.
fn covered(
    generator: &Generator,
    sphere: QuadSphere,
    up: &HashSet<ChunkAddr>,
    direction: DVec3,
) -> bool {
    (0..=coarsest(sphere)).any(|level| {
        ground_chunk(generator, sphere, direction, level).is_some_and(|addr| up.contains(&addr))
    })
}

/// Follows what the renderer would actually be holding.
#[derive(Default)]
struct Renderer {
    up: HashSet<ChunkAddr>,
    by_id: std::collections::HashMap<PatchId, ChunkAddr>,
}

impl Renderer {
    fn apply(&mut self, changes: Vec<TerrainChange>, named: &[(PatchId, ChunkAddr)]) {
        for (id, addr) in named {
            self.by_id.insert(*id, *addr);
        }
        for change in changes {
            match change {
                TerrainChange::Add(id, _) => {
                    if let Some(addr) = self.by_id.get(&id) {
                        self.up.insert(*addr);
                    }
                }
                TerrainChange::Remove(id) => {
                    if let Some(addr) = self.by_id.get(&id) {
                        self.up.remove(addr);
                    }
                }
            }
        }
    }
}

/// A descent from orbit to the ground, a step at a time, with the frame budget
/// a real frame has. The ground under the eye must be drawn the whole way.
#[test]
fn a_descent_never_opens_a_hole_under_the_eye() {
    for bits in [8u32, 10] {
        let (generator, sphere) = body(bits);
        let blocks = sphere.blocks();
        let side = f64::from(blocks.side());
        let point = SurfacePoint::new(topology::Sector::ALL[0], side * 0.37, side * 0.41);
        let direction = DVec3::from(blocks.direction(point));

        let mut terrain = Terrain::new(sphere);
        let start_m = sphere.radius_m() * 3.0;
        let ground_m = generator
            .sample_at(direction.to_array(), topology::BLOCK_M)
            .height_m;
        let end_m = ground_m + 2.0;

        // Settled at the top, so the run starts from a world that is whole.
        terrain.settle(&generator, direction * (sphere.radius_m() + start_m));
        let mut renderer = Renderer::default();
        let mut track = |terrain: &Terrain| -> Vec<(PatchId, ChunkAddr)> { terrain.named() };
        renderer.apply(terrain.drain_changes(), &track(&terrain));
        assert!(
            covered(&generator, sphere, &renderer.up, direction),
            "bits {bits}: not covered even before moving"
        );

        let steps = 400;
        let mut worst_gap = 0;
        let mut gap = 0;
        for i in 1..=steps {
            let t = f64::from(i) / f64::from(steps);
            let height_m = start_m + (end_m - start_m) * t;
            let eye = direction * (sphere.radius_m() + height_m);
            terrain.update(&generator, eye, BUDGET);
            let named = track(&terrain);
            renderer.apply(terrain.drain_changes(), &named);
            if covered(&generator, sphere, &renderer.up, direction) {
                gap = 0;
            } else {
                gap += 1;
                worst_gap = worst_gap.max(gap);
            }
        }
        assert_eq!(
            worst_gap, 0,
            "bits {bits}: the ground under the eye was missing for {worst_gap} frames"
        );
    }
}
