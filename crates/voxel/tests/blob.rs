//! What a chunk costs and what survives a round trip.
//!
//! The blob is the same on disk, on the wire and in a cache, so a chunk that
//! does not come back exactly is three bugs, not one.

use voxel::{CHUNK_CELLS, CHUNK_SIDE, Cell, Chunk, Density, Material};

const ROCK: Material = 4;
const AIR: Material = 0;
const SAND: Material = 1;

fn round_trip(chunk: &Chunk) -> Chunk {
    let blob = chunk.encode();
    let back = Chunk::decode(&blob).expect("a blob this test just wrote");
    assert_eq!(
        &back,
        chunk,
        "a chunk changed on the way through {} bytes",
        blob.len()
    );
    back
}

/// Ground running through the chunk at a slant, so every cell differs.
fn slope() -> Chunk {
    Chunk::from_fn(|[u, v, h]| {
        let ground = 4.0 + (u as f64) * 0.25 + (v as f64) * 0.1;
        let density = Density::from_cells(ground - h as f64);
        Cell {
            density,
            material: if density.solid() { ROCK } else { AIR },
        }
    })
}

#[test]
fn most_of_a_world_costs_four_bytes() {
    // Sky, and rock well under the ground: the common chunk says nothing, and
    // a chunk that says nothing must not be paid for cell by cell.
    for cell in [
        Cell {
            density: Density::AIR,
            material: AIR,
        },
        Cell {
            density: Density::SOLID,
            material: ROCK,
        },
    ] {
        let chunk = Chunk::Uniform(cell);
        assert_eq!(chunk.encode().len(), 4);
        assert!(!chunk.has_surface());
        round_trip(&chunk);
    }
}

#[test]
fn a_chunk_of_one_repeated_cell_collapses_on_the_way_in() {
    // Nothing downstream should have to notice that it could have.
    let chunk = Chunk::from_fn(|_| Cell {
        density: Density::SOLID,
        material: ROCK,
    });
    assert!(matches!(chunk, Chunk::Uniform(_)));
    assert_eq!(chunk.encode().len(), 4);
}

#[test]
fn ground_through_a_chunk_round_trips_cell_for_cell() {
    let chunk = slope();
    assert!(chunk.has_surface(), "the ground runs through this one");
    let back = round_trip(&chunk);
    for h in 0..CHUNK_SIDE {
        for v in 0..CHUNK_SIDE {
            for u in 0..CHUNK_SIDE {
                assert_eq!(back.cell([u, v, h]), chunk.cell([u, v, h]));
            }
        }
    }
}

#[test]
fn two_materials_cost_one_bit_a_cell() {
    // The palette is the whole point: a chunk of rock and air must not spend
    // a byte a cell naming which.
    let blob = slope().encode();
    let header = 1 + 1 + 1 + 2;
    let density = CHUNK_CELLS;
    let indices = CHUNK_CELLS / 8;
    assert_eq!(blob.len(), header + density + indices);
}

#[test]
fn one_material_costs_nothing_a_cell() {
    // Density varies, the cover does not: the indices leave the blob entirely.
    let chunk = Chunk::from_fn(|[u, _, h]| Cell {
        density: Density::from_cells(8.0 - h as f64 + u as f64 * 0.1),
        material: SAND,
    });
    let blob = chunk.encode();
    assert_eq!(blob.len(), 1 + 1 + 1 + 1 + CHUNK_CELLS);
    round_trip(&chunk);
}

#[test]
fn a_full_palette_round_trips() {
    // 256 materials is the most a palette can name, and the index then costs
    // the byte it was meant to save: the point is that it still comes back.
    let chunk = Chunk::from_fn(|at| Cell {
        density: Density::from_cells(8.0 - at[2] as f64),
        material: (voxel::index(at) % 256) as Material,
    });
    round_trip(&chunk);
}

#[test]
fn density_survives_quantising_to_the_precision_it_promises() {
    // A cell's reach is two cells each way; inside that the error must stay
    // under a step, and outside it only the side matters.
    let step = 2.0 * voxel::DENSITY_REACH / 255.0;
    for thousandth in -3000..=3000 {
        let cells = f64::from(thousandth) / 1000.0;
        let back = Density::from_cells(cells).cells();
        if cells.abs() <= voxel::DENSITY_REACH {
            assert!((back - cells).abs() <= step, "{cells} came back {back}");
        }
        assert_eq!(Density::from_cells(cells).solid(), cells > step / 2.0);
    }
}

#[test]
fn a_wrong_blob_is_refused_instead_of_decoded() {
    assert!(Chunk::decode(&[]).is_err());
    assert!(Chunk::decode(&[9, 0, 0, 0]).is_err(), "a newer format");
    assert!(Chunk::decode(&[1, 9]).is_err(), "an unknown kind");
    let mut short = slope().encode();
    short.truncate(short.len() - 1);
    assert!(Chunk::decode(&short).is_err(), "a blob cut short");
}
