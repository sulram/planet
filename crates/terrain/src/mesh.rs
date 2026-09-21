//! A chunk to a mesh a renderer can hold.

use glam::DVec3;
use scene::{TerrainMesh, TerrainVertex};
use topology::{BLOCK_M, QuadSphere, SurfacePoint, vec3};
use voxel::CHUNK_SIDE;
use worldgen::Material;

use crate::address::ChunkAddr;
use crate::read::{Chunks, Lattice};

/// Meshes one chunk, or `None` when no surface passes through it.
///
/// Vertices come out of the mesher in address space, where every block is a
/// unit cube. Here they are placed on the sphere: simulate flat, render
/// spherical.
pub fn mesh(chunks: &Chunks, sphere: QuadSphere, addr: ChunkAddr) -> Option<TerrainMesh> {
    let lattice = Lattice {
        chunks,
        sphere,
        addr,
        outside: 0.0,
    };
    let flat = voxel::mesh(&lattice);
    if flat.is_empty() {
        return None;
    }

    let half = CHUNK_SIDE as f64 / 2.0;
    let centre = place(sphere, addr, [half, half, half]);
    let origin = DVec3::new(centre[0], centre[1], centre[2]);

    let vertices = flat
        .vertices
        .iter()
        .map(|vertex| {
            let local = vertex.position.map(f64::from);
            let at = place(sphere, addr, local);
            let position = [
                (at[0] - centre[0]) as f32,
                (at[1] - centre[1]) as f32,
                (at[2] - centre[2]) as f32,
            ];
            TerrainVertex {
                position,
                normal: spherical_normal(sphere, addr, local, vertex.normal),
                color: color(Material::from_id(vertex.material).unwrap_or(Material::Rock)),
            }
        })
        .collect();

    Some(TerrainMesh {
        origin,
        vertices,
        indices: flat.indices,
        water: None,
        grass: Vec::new(),
    })
}

/// A point in the mesher's cell units, placed on the sphere.
fn place(sphere: QuadSphere, addr: ChunkAddr, local: [f64; 3]) -> [f64; 3] {
    let low = addr.low_column();
    let point = SurfacePoint::new(
        low.sector,
        f64::from(low.u) + local[0] + 0.5,
        f64::from(low.v) + local[1] + 0.5,
    );
    let height_m = (f64::from(addr.low_h()) + local[2] + 0.5) * BLOCK_M;
    sphere.position(sphere.blocks().wrapped(point), height_m)
}

/// The address space normal turned into a world space one, through the local
/// frame: `u` and `v` along the ground, `h` straight up.
fn spherical_normal(
    sphere: QuadSphere,
    addr: ChunkAddr,
    local: [f64; 3],
    normal: [f32; 3],
) -> [f32; 3] {
    let low = addr.low_column();
    let point = sphere.blocks().wrapped(SurfacePoint::new(
        low.sector,
        f64::from(low.u) + local[0] + 0.5,
        f64::from(low.v) + local[1] + 0.5,
    ));
    let frame = sphere.blocks().tangents(point);
    let du = vec3::normalize(frame.du);
    let dv = vec3::normalize(frame.dv);
    let n = normal.map(f64::from);
    let world = vec3::normalize(vec3::add(
        vec3::scale(du, n[0]),
        vec3::add(vec3::scale(dv, n[1]), vec3::scale(frame.up, n[2])),
    ));
    [world[0] as f32, world[1] as f32, world[2] as f32]
}

/// Albedo and gloss of the ground cover.
fn color(material: Material) -> [u8; 4] {
    match material {
        Material::Snow => [236, 238, 240, 70],
        Material::Sand => [206, 192, 150, 8],
        Material::Grass => [104, 138, 70, 0],
        Material::Forest => [92, 128, 66, 0],
        Material::Rock => [118, 112, 106, 12],
        Material::Seabed | Material::Water => [112, 116, 98, 0],
        Material::Regolith => [112, 110, 105, 0],
    }
}
