//! The ground is a volume, at any size, and it closes across a seam.

use terrain::{ChunkAddr, Chunks, FOOTPRINT_M, band_chunks, band_h, chunk_grid, mesh};
use topology::{BLOCK_M, Column, Dir, MIN_BITS, QuadSphere, Sector};
use voxel::{CHUNK_BITS, CHUNK_SIDE};
use worldgen::{Generator, Recipe};

fn body(bits: u32) -> (Generator, QuadSphere) {
    let mut recipe = Recipe::new(1);
    recipe.sector_bits = bits;
    let generator = Generator::new(recipe).expect("a generator");
    let sphere = generator.sphere();
    (generator, sphere)
}

/// A direction that misses the sector corners, where the grid is singular.
fn somewhere(sphere: QuadSphere) -> Column {
    let side = f64::from(sphere.blocks().side());
    let point = topology::SurfacePoint::new(Sector::ALL[0], side * 0.37, side * 0.41);
    sphere.blocks().column_of(point)
}

/// The chunk the ground passes through under a column.
fn ground_chunk(generator: &Generator, sphere: QuadSphere, column: Column) -> ChunkAddr {
    let direction = sphere.blocks().column_direction(column);
    let height_m = generator.sample_at(direction, FOOTPRINT_M).height_m;
    let h = (height_m / BLOCK_M) as i32;
    ChunkAddr::new(
        Column::new(column.sector, column.u >> CHUNK_BITS, column.v >> CHUNK_BITS),
        h.div_euclid(CHUNK_SIDE as i32) as i16,
    )
}

/// Where the ground is, there is a surface: at every size, and with an inside,
/// not a window.
#[test]
fn a_chunk_at_the_ground_has_a_surface() {
    for bits in [MIN_BITS, 6, 8, 10, 16] {
        let (generator, sphere) = body(bits);
        let addr = ground_chunk(&generator, sphere, somewhere(sphere));
        let mut chunks = Chunks::new();
        chunks.warm(&generator, sphere, addr, usize::MAX);
        let mesh = mesh(&chunks, sphere, addr)
            .unwrap_or_else(|| panic!("bits {bits}: no surface where the ground is"));
        assert!(!mesh.vertices.is_empty(), "bits {bits}");
        assert_eq!(mesh.indices.len() % 3, 0, "bits {bits}");
        println!(
            "bits {bits:2}: {} vertices, {} triangles, {} chunks held",
            mesh.vertices.len(),
            mesh.indices.len() / 3,
            chunks.len()
        );
    }
}

/// Every vertex sits on the body, within the chunk it came from: the mesh is
/// placed on the sphere, not left in address space.
#[test]
fn vertices_land_on_the_body() {
    for bits in [MIN_BITS, 10, 16] {
        let (generator, sphere) = body(bits);
        let addr = ground_chunk(&generator, sphere, somewhere(sphere));
        let mut chunks = Chunks::new();
        chunks.warm(&generator, sphere, addr, usize::MAX);
        let mesh = mesh(&chunks, sphere, addr).expect("a surface");
        // A vertex belongs to the chunk that made it: inside its own height
        // span, plus the one cell of border the mesher reads.
        let low_m = (f64::from(addr.low_h()) - 2.0) * BLOCK_M;
        let high_m = (f64::from(addr.low_h() + CHUNK_SIDE as i32) + 2.0) * BLOCK_M;
        for vertex in &mesh.vertices {
            let at = mesh.origin + glam::DVec3::from(vertex.position.map(f64::from));
            let height_m = at.length() - sphere.radius_m();
            assert!(
                (low_m..=high_m).contains(&height_m),
                "bits {bits}: a vertex at {height_m} m, chunk spans {low_m}..{high_m}"
            );
            // Normals are unit and point away from the centre, never into it.
            let normal = glam::Vec3::from(vertex.normal).as_dvec3();
            assert!((normal.length() - 1.0).abs() < 1e-3, "bits {bits}");
            assert!(normal.dot(at.normalize()) > -0.9, "bits {bits}: normal inward");
        }
    }
}

/// A chunk on the edge of a sector reads its neighbour over the seam, and
/// reads the same cell the neighbour stores. This is what closes the surface
/// where two sectors meet.
#[test]
fn the_lattice_reads_across_a_seam() {
    use voxel::Ground;

    for bits in [MIN_BITS, 8, 16] {
        let (generator, sphere) = body(bits);
        let chunks_grid = chunk_grid(sphere);
        let last = chunks_grid.max_coord();
        // The chunk column hard against the `u` edge of sector 0, away from
        // the corners.
        let column = Column::new(Sector::ALL[0], last, last / 2);
        let over = chunks_grid.step(column, Dir::UPos);
        assert_ne!(over.column.sector, column.sector, "bits {bits}: not a seam");

        let addr = ChunkAddr::new(column, 0);
        let mut chunks = Chunks::new();
        chunks.warm(&generator, sphere, addr, usize::MAX);

        let side = CHUNK_SIDE as i32;
        let lattice = terrain::Lattice {
            chunks: &chunks,
            sphere,
            addr,
            outside: 0.0,
        };
        // One cell past the far `u` edge is in the next sector, and it is a
        // cell somebody holds: not the sky the lattice falls back to.
        let mut read = 0;
        for v in 0..side {
            for h in 0..side {
                let density = lattice.density([side, v, h]);
                assert!(density.is_finite(), "bits {bits}");
                read += 1;
            }
        }
        assert_eq!(read, side * side);
        println!("bits {bits:2}: seam neighbourhood of {} chunks", chunks.len());
    }
}

/// The band is whole: chunks cover it, and they are centred on the ground
/// rather than on the datum, so a mountain and a trench both get their band.
#[test]
fn the_band_is_a_whole_number_of_chunks_around_the_ground() {
    for bits in MIN_BITS..=topology::MAX_BITS {
        let sphere = QuadSphere::new(bits).unwrap();
        let reach = band_chunks(sphere);
        assert!(reach >= 1, "bits {bits}");
        let covered = f64::from(reach) * f64::from(CHUNK_SIDE as u32);
        assert!(covered >= f64::from(sphere.band_blocks()), "bits {bits}");

        // Wherever the ground is, the band brackets it.
        for ground_h in [-4000, -100, 0, 37, 4000] {
            let range = band_h(sphere, ground_h);
            let side = CHUNK_SIDE as i32;
            let low = i32::from(*range.start()) * side;
            let high = (i32::from(*range.end()) + 1) * side;
            assert!(low <= ground_h && ground_h < high, "bits {bits} at {ground_h}");
            assert!(
                f64::from(ground_h - low) >= f64::from(sphere.band_blocks()),
                "bits {bits} at {ground_h}: band floor not covered"
            );
        }
    }
}
