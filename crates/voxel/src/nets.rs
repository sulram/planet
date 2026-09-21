//! Surface nets: a density field to a surface, one vertex a cell.
//!
//! The dual of marching cubes, and the reason the terrain layer is smooth.
//! Every cell the ground passes through gets one vertex, placed where the
//! ground crosses that cell's edges, and every edge of the grid that changes
//! sign becomes a quad joining the four cells around it. No table of 256
//! cases, no cracks between cells, and a vertex that can be moved by an edit
//! without changing how many there are.
//!
//! A chunk is meshed with one cell of its low neighbours included, so two
//! chunks side by side produce the same vertices along the edge they share
//! and the surface closes across them. The caller samples that border; this
//! module only asks.

use crate::{CHUNK_SIDE, Material};

/// The lowest corner sampled, one outside the chunk.
const LOW: i32 = -1;
/// One past the highest corner sampled, which is the chunk's far corner.
const HIGH: i32 = CHUNK_SIDE as i32;
/// Corners along one axis, `LOW..=HIGH`.
const CORNERS: usize = CHUNK_SIDE + 2;

/// A vertex of the terrain surface, in cells from the chunk's low corner.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    /// Unit, out of the ground.
    pub normal: [f32; 3],
    pub material: Material,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// What the mesher asks of the world: the ground at a corner, positive inside,
/// and what covers it. Corners run `-1..=CHUNK_SIDE` on every axis.
pub trait Ground {
    fn density(&self, at: [i32; 3]) -> f32;
    fn material(&self, at: [i32; 3]) -> Material;
}

impl<F: Fn([i32; 3]) -> (f32, Material)> Ground for F {
    fn density(&self, at: [i32; 3]) -> f32 {
        self(at).0
    }
    fn material(&self, at: [i32; 3]) -> Material {
        self(at).1
    }
}

/// The 12 edges of a cell, as pairs of its 8 corners. A corner is
/// `i + 2 * j + 4 * k` for its `0` or `1` along each axis.
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

fn corner_offset(corner: usize) -> [f32; 3] {
    [
        (corner & 1) as f32,
        (corner >> 1 & 1) as f32,
        (corner >> 2 & 1) as f32,
    ]
}

fn index(at: [i32; 3]) -> usize {
    let k = |x: i32| (x - LOW) as usize;
    k(at[0]) + CORNERS * (k(at[1]) + CORNERS * k(at[2]))
}

/// Meshes one chunk, with the low border its neighbours share.
pub fn mesh(ground: &impl Ground) -> Mesh {
    // Reading every corner once is the whole cost: a cell wants eight and
    // shares all of them.
    let mut density = vec![0.0f32; CORNERS * CORNERS * CORNERS];
    let mut material = vec![0 as Material; CORNERS * CORNERS * CORNERS];
    for z in LOW..=HIGH {
        for y in LOW..=HIGH {
            for x in LOW..=HIGH {
                let at = [x, y, z];
                let i = index(at);
                density[i] = ground.density(at);
                material[i] = ground.material(at);
            }
        }
    }

    let mut mesh = Mesh::default();
    // One vertex a cell, or none. `u32::MAX` is none.
    let cells = CORNERS - 1;
    let mut vertex_at = vec![u32::MAX; cells * cells * cells];
    let cell_index = |at: [i32; 3]| {
        let k = |x: i32| (x - LOW) as usize;
        k(at[0]) + cells * (k(at[1]) + cells * k(at[2]))
    };

    for z in LOW..HIGH {
        for y in LOW..HIGH {
            for x in LOW..HIGH {
                let cell = [x, y, z];
                let mut around = [0.0f32; 8];
                for (corner, value) in around.iter_mut().enumerate() {
                    let offset = corner_offset(corner);
                    *value = density[index([
                        cell[0] + offset[0] as i32,
                        cell[1] + offset[1] as i32,
                        cell[2] + offset[2] as i32,
                    ])];
                }
                let solid = around[0] > 0.0;
                if around.iter().all(|d| (*d > 0.0) == solid) {
                    continue;
                }

                // The vertex sits where the ground crosses this cell's edges,
                // which is what keeps the surface smooth instead of stepped.
                let mut sum = [0.0f32; 3];
                let mut crossings = 0.0f32;
                for (a, b) in EDGES {
                    let (da, db) = (around[a], around[b]);
                    if (da > 0.0) == (db > 0.0) {
                        continue;
                    }
                    let t = da / (da - db);
                    let (oa, ob) = (corner_offset(a), corner_offset(b));
                    for axis in 0..3 {
                        sum[axis] += oa[axis] + t * (ob[axis] - oa[axis]);
                    }
                    crossings += 1.0;
                }

                // The gradient across the cell points into the ground, so out
                // of it is the other way. Taken from the corners we already
                // have rather than from four more samples.
                let face = |bit: usize| {
                    let (mut high, mut low) = (0.0, 0.0);
                    for (corner, value) in around.iter().enumerate() {
                        if corner >> bit & 1 == 1 {
                            high += value;
                        } else {
                            low += value;
                        }
                    }
                    low - high
                };
                let gradient = [face(0), face(1), face(2)];
                let length = (gradient[0] * gradient[0]
                    + gradient[1] * gradient[1]
                    + gradient[2] * gradient[2])
                    .sqrt();
                let normal = if length > 0.0 {
                    [
                        gradient[0] / length,
                        gradient[1] / length,
                        gradient[2] / length,
                    ]
                } else {
                    [0.0, 1.0, 0.0]
                };

                // What covers the ground here is what the deepest corner of
                // the ground says, so a shore does not wear the sea's cover.
                let deepest = (0..8)
                    .filter(|corner| around[*corner] > 0.0)
                    .max_by(|a, b| around[*a].total_cmp(&around[*b]));
                let cover = deepest.map_or(0, |corner| {
                    let offset = corner_offset(corner);
                    material[index([
                        cell[0] + offset[0] as i32,
                        cell[1] + offset[1] as i32,
                        cell[2] + offset[2] as i32,
                    ])]
                });

                vertex_at[cell_index(cell)] = mesh.vertices.len() as u32;
                mesh.vertices.push(Vertex {
                    position: [
                        cell[0] as f32 + sum[0] / crossings,
                        cell[1] as f32 + sum[1] / crossings,
                        cell[2] as f32 + sum[2] / crossings,
                    ],
                    normal,
                    material: cover,
                });
            }
        }
    }

    // Every grid edge that changes sign is a quad of the four cells that meet
    // along it. Walking the edges instead of the cells is what makes the
    // surface closed by construction.
    for z in LOW..HIGH {
        for y in LOW..HIGH {
            for x in LOW..HIGH {
                let at = [x, y, z];
                let here = density[index(at)] > 0.0;
                for axis in 0..3 {
                    let mut ahead = at;
                    ahead[axis] += 1;
                    if (density[index(ahead)] > 0.0) == here {
                        continue;
                    }
                    // The four cells around the edge differ by -1 on the two
                    // axes the edge does not run along.
                    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                    let mut quad = [u32::MAX; 4];
                    let mut inside = true;
                    for (corner, slot) in quad.iter_mut().enumerate() {
                        let mut cell = at;
                        cell[u] -= (corner & 1) as i32;
                        cell[v] -= (corner >> 1 & 1) as i32;
                        if cell.iter().any(|k| *k < LOW || *k >= HIGH) {
                            inside = false;
                            break;
                        }
                        *slot = vertex_at[cell_index(cell)];
                        inside &= *slot != u32::MAX;
                    }
                    if !inside {
                        continue;
                    }
                    // Wind so the face turns its front to the air.
                    let [a, b, c, d] = if here {
                        [quad[0], quad[1], quad[3], quad[2]]
                    } else {
                        [quad[0], quad[2], quad[3], quad[1]]
                    };
                    mesh.indices.extend([a, b, c, a, c, d]);
                }
            }
        }
    }
    mesh
}
