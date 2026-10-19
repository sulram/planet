//! The sides of a model's voxels that show, and how shut in each corner is:
//! the same rule the cells are meshed by (`voxel::faces`), so a model and a
//! volume shade alike. A side is one quad; sides between two voxels are
//! never drawn, and sides are not merged, so occlusion stays per corner.

use crate::{Grid, Palette};

/// One side of a voxel: an axis and which way along it the side faces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Face {
    /// `0` for `x`, `1` for `y`, `2` for `z`.
    pub axis: usize,
    pub positive: bool,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face {
            axis: 0,
            positive: true,
        },
        Face {
            axis: 0,
            positive: false,
        },
        Face {
            axis: 1,
            positive: true,
        },
        Face {
            axis: 1,
            positive: false,
        },
        Face {
            axis: 2,
            positive: true,
        },
        Face {
            axis: 2,
            positive: false,
        },
    ];

    fn normal(self) -> [i32; 3] {
        let mut n = [0; 3];
        n[self.axis] = if self.positive { 1 } else { -1 };
        n
    }
}

/// One visible side of one voxel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Quad {
    pub voxel: [i32; 3],
    pub face: Face,
    /// Index into the palette.
    pub color: u8,
    /// How open each corner is, in the order of [`Quad::corners`]: `3` open,
    /// `0` shut in on both edges.
    pub open: [u8; 4],
}

/// The axis a side faces, then the two across it in right handed order.
fn axes(face: Face) -> (usize, usize, usize) {
    let a = face.axis;
    (a, (a + 1) % 3, (a + 2) % 3)
}

impl Quad {
    /// The corners of the side as lattice points, counter clockwise seen from
    /// outside.
    pub fn corners(self) -> [[i32; 3]; 4] {
        let (a, b, c) = axes(self.face);
        let mut base = self.voxel;
        if self.face.positive {
            base[a] += 1;
        }
        let at = |sb: i32, sc: i32| {
            let mut p = base;
            p[b] += sb;
            p[c] += sc;
            p
        };
        let square = [at(0, 0), at(1, 0), at(1, 1), at(0, 1)];
        if self.face.positive {
            square
        } else {
            [square[0], square[3], square[2], square[1]]
        }
    }

    /// Whether the side is split along the diagonal from the second corner to
    /// the fourth instead of from the first to the third: along the brighter
    /// one, occlusion shades evenly instead of in a crease.
    pub fn flipped(self) -> bool {
        let [a, b, c, d] = self.open.map(u32::from);
        a + c < b + d
    }
}

/// How open a corner is from the three voxels that touch it in the layer the
/// side looks into: the two along its edges and the one across. Shut in on
/// both edges, the one across cannot be seen and the corner is dark.
fn openness(face: Face, solid: impl Fn(i32, i32) -> bool) -> [u8; 4] {
    let corner = |sb: i32, sc: i32| {
        let (b, c, bc) = (solid(sb, 0), solid(0, sc), solid(sb, sc));
        if b && c {
            0
        } else {
            3 - u8::from(b) - u8::from(c) - u8::from(bc)
        }
    };
    let square = [corner(-1, -1), corner(1, -1), corner(1, 1), corner(-1, 1)];
    if face.positive {
        square
    } else {
        [square[0], square[3], square[2], square[1]]
    }
}

/// The visible sides of every voxel of a grid.
pub fn quads(grid: &Grid) -> Vec<Quad> {
    let mut quads = Vec::new();
    let solid = |p: [i32; 3]| grid.at(p) != 0;
    for z in 0..grid.size[2] as i32 {
        for y in 0..grid.size[1] as i32 {
            for x in 0..grid.size[0] as i32 {
                let at = [x, y, z];
                let color = grid.at(at);
                if color == 0 {
                    continue;
                }
                for face in Face::ALL {
                    let n = face.normal();
                    let out = [at[0] + n[0], at[1] + n[1], at[2] + n[2]];
                    if solid(out) {
                        continue;
                    }
                    let (_, b, c) = axes(face);
                    let open = openness(face, |sb, sc| {
                        let mut p = out;
                        p[b] += sb;
                        p[c] += sc;
                        solid(p)
                    });
                    quads.push(Quad {
                        voxel: at,
                        face,
                        color,
                        open,
                    });
                }
            }
        }
    }
    quads
}

/// A corner of a side, as a renderer takes it: in voxels, from the middle of
/// the model's footprint on its bottom face, `z` up.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    /// Of the side this vertex is a corner of.
    pub normal: [f32; 3],
    /// `rgb` of the voxel's colour as the palette has it, and `255`.
    pub color: [u8; 4],
    /// How much of the sky the corner sees, `0` to `255`.
    pub sky: u8,
}

/// A model's visible sides as triangles: four vertices a side, six indices.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// Meshes a grid with its palette.
pub fn mesh(grid: &Grid, palette: &Palette) -> Mesh {
    let quads = quads(grid);
    let mut out = Mesh {
        vertices: Vec::with_capacity(quads.len() * 4),
        indices: Vec::with_capacity(quads.len() * 6),
    };
    let origin = [grid.size[0] as f32 / 2.0, grid.size[1] as f32 / 2.0, 0.0];
    for quad in quads {
        let first = out.vertices.len() as u32;
        let normal = quad.face.normal().map(|n| n as f32);
        let [r, g, b, _] = palette[usize::from(quad.color)];
        for (corner, open) in quad.corners().into_iter().zip(quad.open) {
            out.vertices.push(Vertex {
                position: [0, 1, 2].map(|i| corner[i] as f32 - origin[i]),
                normal,
                color: [r, g, b, 255],
                sky: open * 85,
            });
        }
        let order: [u32; 6] = if quad.flipped() {
            [1, 2, 3, 1, 3, 0]
        } else {
            [0, 1, 2, 0, 2, 3]
        };
        out.indices.extend(order.map(|i| first + i));
    }
    out
}
