//! What the mesher owes a shape it is given.
//!
//! A surface is hard to eyeball and easy to check: a plane is flat, a sphere
//! is round, and a closed shape has no edge used once. Each of these fails on
//! a different mistake, which is why there are three.

use voxel::{CHUNK_SIDE, Material, Mesh, Vertex, mesh};

const ROCK: Material = 4;

fn build(density: impl Fn([f32; 3]) -> f32) -> Mesh {
    mesh(&|at: [i32; 3]| {
        let p = [at[0] as f32, at[1] as f32, at[2] as f32];
        (density(p), ROCK)
    })
}

fn normal_of(mesh: &Mesh, triangle: usize) -> [f32; 3] {
    let at = |k: usize| mesh.vertices[mesh.indices[triangle * 3 + k] as usize].position;
    let (a, b, c) = (at(0), at(1), at(2));
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len == 0.0 {
        n
    } else {
        [n[0] / len, n[1] / len, n[2] / len]
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[test]
fn ground_with_no_surface_in_it_makes_nothing() {
    assert!(build(|_| 1.0).is_empty(), "solid rock is not a surface");
    assert!(build(|_| -1.0).is_empty(), "nor is open sky");
}

#[test]
fn a_flat_ground_comes_out_flat() {
    // Every vertex on the plane, every normal straight up, and no step: this
    // is the whole reason the terrain layer is surface nets and not cubes.
    let ground_z = 8.3;
    let mesh = build(|p| ground_z - p[2]);
    assert!(!mesh.is_empty());
    for Vertex {
        position, normal, ..
    } in &mesh.vertices
    {
        assert!((position[2] - ground_z).abs() < 0.01, "{position:?}");
        assert!(dot(*normal, [0.0, 0.0, 1.0]) > 0.999, "{normal:?}");
    }
}

#[test]
fn a_round_ground_comes_out_round() {
    let middle = CHUNK_SIDE as f32 / 2.0;
    let radius = 5.0;
    let mesh = build(|p| {
        let d = [p[0] - middle, p[1] - middle, p[2] - middle];
        radius - (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    });
    assert!(!mesh.is_empty());
    for Vertex {
        position, normal, ..
    } in &mesh.vertices
    {
        let d = [
            position[0] - middle,
            position[1] - middle,
            position[2] - middle,
        ];
        let out = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!(
            (out - radius).abs() < 0.2,
            "{position:?} is {out} from the middle"
        );
        // The normal of a ball points away from its middle.
        let radial = [d[0] / out, d[1] / out, d[2] / out];
        assert!(dot(*normal, radial) > 0.9, "{normal:?} against {radial:?}");
    }
}

#[test]
fn a_face_turns_its_front_to_the_air() {
    // Winding and the normals have to agree, or the ball is inside out and
    // nothing says so until the light is wrong.
    let middle = CHUNK_SIDE as f32 / 2.0;
    let mesh = build(|p| {
        let d = [p[0] - middle, p[1] - middle, p[2] - middle];
        5.0 - (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    });
    for triangle in 0..mesh.indices.len() / 3 {
        let face = normal_of(&mesh, triangle);
        let vertex = mesh.vertices[mesh.indices[triangle * 3] as usize].normal;
        assert!(
            dot(face, vertex) > 0.0,
            "triangle {triangle} faces {face:?}, vertex {vertex:?}"
        );
    }
}

#[test]
fn a_closed_shape_has_no_loose_edge() {
    // Every edge of a closed surface belongs to exactly two triangles. A
    // crack, a hole or a quad wound two ways all show up here and nowhere
    // else. The ball is well inside the chunk, so nothing is cut off.
    let middle = CHUNK_SIDE as f32 / 2.0;
    let mesh = build(|p| {
        let d = [p[0] - middle, p[1] - middle, p[2] - middle];
        4.0 - (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    });
    let mut edges = std::collections::HashMap::new();
    for triangle in mesh.indices.chunks_exact(3) {
        for k in 0..3 {
            let (a, b) = (triangle[k], triangle[(k + 1) % 3]);
            *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    let loose: Vec<_> = edges.iter().filter(|(_, n)| **n != 2).collect();
    assert!(
        loose.is_empty(),
        "{} edges are not shared by two faces",
        loose.len()
    );
}

#[test]
fn two_chunks_side_by_side_agree_along_the_edge_they_share() {
    // The low border is meshed by both, so the surface closes across them.
    // If this drifts, every chunk boundary in the world is a crack.
    let ground = |p: [f32; 3]| 6.0 + p[0] * 0.2 - p[2];
    let here = build(ground);
    let side = CHUNK_SIDE as f32;
    let next = build(|p| ground([p[0] + side, p[1], p[2]]));
    let seam: Vec<_> = here
        .vertices
        .iter()
        .filter(|v| v.position[0] > side - 1.0)
        .map(|v| [v.position[0] - side, v.position[1], v.position[2]])
        .collect();
    assert!(!seam.is_empty(), "no vertices at the seam to compare");
    for position in seam {
        let matched = next
            .vertices
            .iter()
            .any(|v| (0..3).all(|axis| (v.position[axis] - position[axis]).abs() < 1e-4));
        assert!(matched, "{position:?} has no twin in the next chunk");
    }
}
