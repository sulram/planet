//! A 3D model on the client (`part::Shape::Glb`): its file weighed against
//! the budget, read at rest, and its meshes laid into its part's unit box as
//! one mesh drawn with pictures (`scene::TexturedMesh`).
//!
//! The budget bounds what one model costs whatever its file: its vertices
//! bound what is read of it, its triangles what is built of it and drawn,
//! its texels the memory of its pictures, its materials the draws it adds
//! to a pass. One function checks
//! it, [`weigh`], from the document and the pictures' headers alone: the
//! front end runs it before the bytes leave for the world, and the client
//! again when a file arrives, since a file in the world may come from any
//! client.

use std::sync::Arc;

use glb::{Alpha, Glb, Model, Weight};
use part::{MAX_SCALE, MIN_SCALE};
use scene::{TexturedMesh, TexturedRun, TexturedVertex};
use topology::BLOCK_M;

/// The most triangles a 3D model draws, a material seen from both sides
/// counting its own twice: what a phone draws of one model among many at
/// sixty frames a second, a detailed figure or a small building. A part is
/// drawn whole at every distance it is drawn from, with no coarser level.
pub const MAX_TRIANGLES: u64 = 65_536;

/// The most texels of a model's pictures, each picture counted once: one of
/// 2048 × 2048, 16 MiB as RGBA and a third more with its mip chain, what a
/// screen shows of a model a few metres away.
pub const MAX_TEXELS: u64 = 2048 * 2048;

/// The most materials a model draws with: each is a draw of its own, every
/// frame, in the solid pass and in every shadow cascade it reaches.
pub const MAX_MATERIALS: u32 = 32;

/// The most vertices a model's meshes hold, each mesh counted once: three
/// corners for each of [`MAX_TRIANGLES`]. A file whose vertices its
/// triangles use is never over it; one over it carries vertices nothing
/// draws, and reading them would be work no other number bounds.
pub const MAX_VERTICES: u64 = 196_608;

/// The longest side a model's picture is decoded at. Within the texels, a
/// long strip is the only picture wider than 2048, and it is halved to the
/// side every device takes.
pub const PICTURE_SIDE: u32 = 4096;

/// Why a 3D model is not drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModelRefusal {
    /// Not a binary glTF this engine reads, or one with no triangle.
    Broken,
    /// Over [`MAX_TRIANGLES`].
    Triangles,
    /// Over [`MAX_TEXELS`].
    Texels,
    /// Over [`MAX_MATERIALS`].
    Materials,
    /// Over [`MAX_VERTICES`].
    Vertices,
}

impl ModelRefusal {
    /// The code a front end words it by.
    pub fn code(self) -> &'static str {
        match self {
            ModelRefusal::Broken => "broken",
            ModelRefusal::Triangles => "triangles",
            ModelRefusal::Texels => "texels",
            ModelRefusal::Materials => "materials",
            ModelRefusal::Vertices => "vertices",
        }
    }
}

/// Weighs a `.glb` against the budget, building nothing: what it would
/// draw, or why it would not.
pub fn weigh(bytes: &[u8]) -> Result<Weight, ModelRefusal> {
    let glb = Glb::open(bytes).map_err(|_| ModelRefusal::Broken)?;
    let weight = glb.weigh();
    within(weight)?;
    Ok(weight)
}

fn within(weight: Weight) -> Result<(), ModelRefusal> {
    if weight.triangles == 0 {
        Err(ModelRefusal::Broken)
    } else if weight.triangles + weight.both_sides > MAX_TRIANGLES {
        Err(ModelRefusal::Triangles)
    } else if weight.texels > MAX_TEXELS {
        Err(ModelRefusal::Texels)
    } else if weight.materials > MAX_MATERIALS {
        Err(ModelRefusal::Materials)
    } else if weight.vertices > MAX_VERTICES {
        Err(ModelRefusal::Vertices)
    } else {
        Ok(())
    }
}

/// A 3D model read and laid into its part's unit box, its pictures still to
/// be decoded.
pub(crate) struct Laid {
    /// Its size as a hand places it, in blocks: its own metres.
    pub size: [f64; 3],
    /// Its mesh, each run naming its picture by its place in `pictures`,
    /// with no image yet.
    pub mesh: TexturedMesh,
    /// The bytes of each picture, PNG, JPEG or WebP.
    pub pictures: Vec<Arc<[u8]>>,
}

/// Reads a `.glb` within the budget and lays it into the unit box.
pub(crate) fn read(bytes: &[u8]) -> Result<Laid, ModelRefusal> {
    let glb = Glb::open(bytes).map_err(|_| ModelRefusal::Broken)?;
    within(glb.weigh())?;
    let model = Model::read(&glb).map_err(|_| ModelRefusal::Broken)?;
    Ok(laid(model))
}

/// The thinnest a model is along any axis, metres: a flat one, a sheet, is
/// as thin as a part may be.
const THIN_M: f32 = (MIN_SCALE * BLOCK_M) as f32;

/// A model in the part's unit box: `x` and `y` across it from the middle,
/// `z` up from the bottom, each axis divided by the model's length along
/// it. The file's frame, `y` up and its front toward `+z`, is turned into
/// the part's, `z` up and its front toward `-y`, as a picture's. A material
/// seen from both sides gets its triangles again, wound and facing the
/// other way.
fn laid(model: Model) -> Laid {
    let turn = |[x, y, z]: [f32; 3]| [x, -z, y];
    let (min, max) = (model.min, model.max);
    let low = [min[0], -max[2], min[1]];
    let high = [max[0], -min[2], max[1]];
    let side = [0, 1, 2].map(|i| (high[i] - low[i]).max(THIN_M));
    let anchor = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0, low[2]];

    // Every corner belongs to one primitive, and so to one material.
    let mut material_of = vec![0; model.vertices.len()];
    for run in &model.runs {
        for &index in &model.indices[run.indices.start as usize..run.indices.end as usize] {
            material_of[index as usize] = run.material;
        }
    }
    let mut vertices: Vec<TexturedVertex> = model
        .vertices
        .iter()
        .zip(&material_of)
        .map(|(vertex, &material)| {
            let position = turn(vertex.position);
            // A normal is carried by the inverse transpose of the division
            // by each side: times it.
            let normal = turn(vertex.normal);
            let normal = glam::Vec3::from([0, 1, 2].map(|i| normal[i] * side[i]))
                .normalize_or_zero()
                .to_array();
            let tint = model.materials[material].color;
            let color = [0, 1, 2, 3].map(|i| vertex.color[i] * tint[i]);
            TexturedVertex {
                position: [0, 1, 2].map(|i| (position[i] - anchor[i]) / side[i]),
                normal,
                uv: vertex.uv,
                color: [
                    srgb(color[0]),
                    srgb(color[1]),
                    srgb(color[2]),
                    unit(color[3]),
                ],
            }
        })
        .collect();

    let mut indices = Vec::with_capacity(model.indices.len());
    let mut runs = Vec::with_capacity(model.runs.len());
    // The corner seen from behind of each corner, once made.
    let mut behind = vec![u32::MAX; vertices.len()];
    for run in &model.runs {
        let start = indices.len() as u32;
        let own = &model.indices[run.indices.start as usize..run.indices.end as usize];
        indices.extend_from_slice(own);
        let material = model.materials[run.material];
        if material.both_sides {
            for triangle in own.chunks_exact(3) {
                for &index in [triangle[0], triangle[2], triangle[1]].iter() {
                    let back = &mut behind[index as usize];
                    if *back == u32::MAX {
                        let mut vertex = vertices[index as usize];
                        vertex.normal = vertex.normal.map(|n| -n);
                        *back = vertices.len() as u32;
                        vertices.push(vertex);
                    }
                    indices.push(*back);
                }
            }
        }
        runs.push(TexturedRun {
            indices: start..indices.len() as u32,
            image: material.picture,
            // A blend is drawn as a cutout at its middle, as an avatar's
            // is: blending would sort every triangle every frame.
            cutoff: match material.alpha {
                Alpha::Opaque => 0.0,
                Alpha::Mask(cutoff) => cutoff,
                Alpha::Blend => 0.5,
            },
        });
    }

    // Metres become blocks; a model larger than a part may be is scaled
    // down whole, its proportion kept.
    let size = side.map(|m| f64::from(m) / BLOCK_M);
    let over = size.iter().copied().fold(1.0, f64::max) / MAX_SCALE;
    let size = size.map(|n| (n / over.max(1.0)).max(MIN_SCALE));
    Laid {
        size,
        mesh: TexturedMesh {
            vertices,
            indices,
            runs,
            images: Vec::new(),
        },
        pictures: model
            .pictures
            .into_iter()
            .map(|picture| Arc::from(picture.bytes))
            .collect(),
    }
}

/// A linear colour channel as an sRGB byte, as the shader reads it back.
fn srgb(linear: f32) -> u8 {
    unit(linear.clamp(0.0, 1.0).powf(1.0 / 2.2))
}

fn unit(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use glb::{Material, Run, Vertex};

    fn vertex(position: [f32; 3], normal: [f32; 3]) -> Vertex {
        Vertex {
            position,
            normal,
            uv: [0.0, 0.0],
            color: [1.0; 4],
        }
    }

    /// One triangle standing in the file's `xy` plane, facing its front,
    /// 2 m wide and 1 m tall, in a material seen from both sides.
    fn standing() -> Model {
        Model {
            vertices: vec![
                vertex([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
                vertex([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
                vertex([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
            ],
            indices: vec![0, 1, 2],
            runs: vec![Run {
                indices: 0..3,
                material: 0,
            }],
            materials: vec![Material {
                color: [0.5, 1.0, 1.0, 1.0],
                picture: None,
                alpha: Alpha::Mask(0.25),
                both_sides: true,
            }],
            pictures: Vec::new(),
            min: [-1.0, 0.0, 0.0],
            max: [1.0, 1.0, 0.0],
        }
    }

    #[test]
    fn a_model_stands_in_the_unit_box_facing_the_front() {
        let laid = laid(standing());
        let mesh = &laid.mesh;
        assert_eq!(laid.size, [4.0, MIN_SCALE, 2.0]);
        let positions: Vec<_> = mesh.vertices[..3].iter().map(|v| v.position).collect();
        assert_eq!(
            positions,
            [[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.0, 1.0]]
        );
        // The file's front, +z, is the part's -y.
        assert_eq!(mesh.vertices[0].normal, [0.0, -1.0, 0.0]);
        // Linear half is the sRGB byte the shader reads back as half.
        assert_eq!(mesh.vertices[0].color, [186, 255, 255, 255]);
        assert_eq!(mesh.runs.len(), 1);
        assert_eq!(mesh.runs[0].cutoff, 0.25);
    }

    #[test]
    fn both_sides_are_drawn_as_two_faces() {
        let mesh = laid(standing()).mesh;
        assert_eq!(mesh.indices.len(), 6);
        assert_eq!(mesh.runs[0].indices, 0..6);
        let face = |at: usize| {
            let [a, b, c] = [0, 1, 2]
                .map(|k| glam::Vec3::from(mesh.vertices[mesh.indices[at + k] as usize].position));
            (b - a).cross(c - a).normalize()
        };
        // Counter clockwise from the front: the front faces -y, the back +y,
        // and each face's corners say so.
        assert_eq!(face(0), glam::Vec3::NEG_Y);
        assert_eq!(face(3), glam::Vec3::Y);
        assert_eq!(
            mesh.vertices[mesh.indices[3] as usize].normal,
            [0.0, 1.0, 0.0]
        );
    }

    #[test]
    fn the_budget_says_what_is_over() {
        let light = Weight {
            triangles: 100,
            both_sides: 0,
            texels: 1024 * 1024,
            materials: 2,
            vertices: 300,
        };
        assert_eq!(within(light), Ok(()));
        let twice = Weight {
            triangles: MAX_TRIANGLES / 2 + 1,
            both_sides: MAX_TRIANGLES / 2 + 1,
            ..light
        };
        assert_eq!(within(twice), Err(ModelRefusal::Triangles));
        let texels = Weight {
            texels: MAX_TEXELS + 1,
            ..light
        };
        assert_eq!(within(texels), Err(ModelRefusal::Texels));
        let materials = Weight {
            materials: MAX_MATERIALS + 1,
            ..light
        };
        assert_eq!(within(materials), Err(ModelRefusal::Materials));
        let vertices = Weight {
            vertices: MAX_VERTICES + 1,
            ..light
        };
        assert_eq!(within(vertices), Err(ModelRefusal::Vertices));
        let empty = Weight {
            triangles: 0,
            ..light
        };
        assert_eq!(within(empty), Err(ModelRefusal::Broken));
        assert_eq!(weigh(b"not a glTF"), Err(ModelRefusal::Broken));
    }
}
