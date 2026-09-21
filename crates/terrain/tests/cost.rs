//! What the streamer costs, in the open.

use std::time::Instant;

use glam::DVec3;
use terrain::{BUDGET, CHUNK_M, Terrain};
use topology::{QuadSphere, SurfacePoint};
use worldgen::{Generator, Recipe};

#[test]
fn what_one_update_costs() {
    for bits in [4u32, 8, 10, 16] {
        let mut recipe = Recipe::new(1);
        recipe.sector_bits = bits;
        let generator = Generator::new(recipe).expect("a generator");
        let sphere: QuadSphere = generator.sphere();
        let blocks = sphere.blocks();
        let side = f64::from(blocks.side());
        let point = SurfacePoint::new(topology::Sector::ALL[0], side * 0.37, side * 0.41);
        let direction = blocks.direction(point);
        let ground_m = generator.sample_at(direction, 0.5).height_m;
        let eye = DVec3::from(direction) * (sphere.radius_m() + ground_m + 2.0);

        let mut terrain = Terrain::new(sphere);

        // What a settled world costs to build from nothing, and what one
        // chunk of it costs on average.
        let cold = Instant::now();
        let drawn = terrain.settle(&generator, eye);
        let cold_ms = cold.elapsed().as_secs_f64() * 1e3;
        let built = terrain.held_chunks().max(1);
        terrain.drain_changes();

        // Standing still, with nothing left to build.
        let warm = Instant::now();
        terrain.update(&generator, eye, BUDGET);
        let warm_ms = warm.elapsed().as_secs_f64() * 1e3;

        // A step far enough to leave the chunk, which is what makes a frame
        // work out a new wanted set and start a queue.
        let across = DVec3::from(blocks.direction(point))
            .cross(DVec3::Z)
            .normalize();
        let moved = eye + across * (CHUNK_M + 1.0);
        // With no budget at all, what is left is the wanted set: the scan,
        // the eviction and the sort. That is the floor a frame pays.
        let scan = Instant::now();
        terrain.update(&generator, moved, 0);
        let scan_ms = scan.elapsed().as_secs_f64() * 1e3;

        let moved = eye + across * (CHUNK_M * 2.0 + 1.0);
        let step = Instant::now();
        terrain.update(&generator, moved, BUDGET);
        let step_ms = step.elapsed().as_secs_f64() * 1e3;
        let queued = terrain.queued();

        println!(
            "bits {bits:2}: {:4} drawn, {:5} held | cold {cold_ms:7.1} ms ({:.2} ms a chunk) | still {warm_ms:5.2} ms | scan {scan_ms:5.2} ms | a chunk over {step_ms:5.2} ms, {queued} queued",
            drawn.len(),
            terrain.held_chunks(),
            cold_ms / built as f64,
        );
        assert!(
            step_ms < 12.0,
            "bits {bits}: a frame that moves costs {step_ms} ms of a 12 ms budget"
        );
    }
}
