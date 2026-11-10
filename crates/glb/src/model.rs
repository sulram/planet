//! A glTF at rest as one static model: every mesh its scene places, where
//! its node stands, in the file's own frame and metres, its triangles
//! gathered by material.

use std::ops::Range;

use glam::{Mat3, Mat4, Vec3};
use gltf::mesh::Mode;

use crate::{Error, Glb, Hierarchy};

/// A corner of a triangle, in the file's frame: metres, `y` up, the front
/// toward `+z`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Where on its material's picture, `(0, 0)` its top left corner.
    pub uv: [f32; 2],
    /// `COLOR_0`, linear RGBA; white where the file says none.
    pub color: [f32; 4],
}

/// How a material's alpha is read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alpha {
    /// Ignored.
    Opaque,
    /// Drawn where alpha reaches the cutoff, and not under it.
    Mask(f32),
    /// Blended with what is behind.
    Blend,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// The base colour factor, linear RGBA.
    pub color: [f32; 4],
    /// Its base colour picture, by index in [`Model::pictures`].
    pub picture: Option<usize>,
    pub alpha: Alpha,
    /// Seen from behind as from the front: a leaf, a sheet.
    pub both_sides: bool,
}

/// glTF's material for a primitive that names none.
const PLAIN: Material = Material {
    color: [1.0; 4],
    picture: None,
    alpha: Alpha::Opaque,
    both_sides: false,
};

/// The triangles of one material, a range of [`Model::indices`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub indices: Range<u32>,
    pub material: usize,
}

/// A picture as the file holds it, PNG or JPEG, with the size its header
/// says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Every mesh a GLB's scene places, at rest, as one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    /// Three a triangle, wound counter clockwise seen from its front.
    pub indices: Vec<u32>,
    /// One a material drawn.
    pub runs: Vec<Run>,
    pub materials: Vec<Material>,
    pub pictures: Vec<Picture>,
    /// The least and the most of every position, metres.
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// What a model costs to draw, read from the document and the pictures'
/// headers before anything is built.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Weight {
    /// Triangles placed: a mesh the scene places twice counts twice.
    pub triangles: u64,
    /// Of those, the ones in a material seen from both sides.
    pub both_sides: u64,
    /// Texels of the base colour pictures drawn, each picture once.
    pub texels: u64,
    /// Materials drawn.
    pub materials: u32,
}

impl Glb {
    /// What [`Model::read`] would build of this file, counted without
    /// building it.
    pub fn weigh(&self) -> Weight {
        let hierarchy = Hierarchy::read(&self.document);
        let mut weight = Weight::default();
        // Whether each material, the plain one last, and each picture is
        // counted yet.
        let plain = self.document.materials().len();
        let mut materials = vec![false; plain + 1];
        let mut pictures = vec![false; self.document.images().len()];
        for node in self.placed(&hierarchy) {
            let Some(mesh) = node.mesh() else {
                continue;
            };
            for primitive in mesh.primitives() {
                if primitive.mode() != Mode::Triangles {
                    continue;
                }
                let corners = match primitive.indices() {
                    Some(indices) => indices.count(),
                    None => primitive
                        .get(&gltf::Semantic::Positions)
                        .map_or(0, |positions| positions.count()),
                };
                let triangles = (corners / 3) as u64;
                let material = primitive.material();
                weight.triangles += triangles;
                if material.double_sided() {
                    weight.both_sides += triangles;
                }
                if !std::mem::replace(&mut materials[material.index().unwrap_or(plain)], true) {
                    weight.materials += 1;
                }
                if let Some(image) = base_image(&material)
                    && !std::mem::replace(&mut pictures[image.index()], true)
                    && let Some(picture) = self.sized(&image)
                {
                    weight.texels += u64::from(picture.width) * u64::from(picture.height);
                }
            }
        }
        weight
    }

    /// The nodes the scene places that hold a mesh, parents first: those of
    /// the default scene, else of the first, else every root's.
    fn placed(&self, hierarchy: &Hierarchy) -> Vec<gltf::Node<'_>> {
        let nodes: Vec<gltf::Node<'_>> = self.document.nodes().collect();
        let scene = self
            .document
            .default_scene()
            .or_else(|| self.document.scenes().next());
        let mut stack: Vec<usize> = match scene {
            Some(scene) => scene.nodes().map(|node| node.index()).collect(),
            None => (0..nodes.len())
                .filter(|&i| hierarchy.parent[i].is_none())
                .collect(),
        };
        stack.reverse();
        let mut seen = vec![false; nodes.len()];
        let mut placed = Vec::new();
        while let Some(index) = stack.pop() {
            if std::mem::replace(&mut seen[index], true) {
                continue;
            }
            let node = &nodes[index];
            if node.mesh().is_some() {
                placed.push(node.clone());
            }
            let first = stack.len();
            stack.extend(node.children().map(|child| child.index()));
            stack[first..].reverse();
        }
        placed
    }

    /// A picture the file holds whose size its header says. One it does
    /// not, or one outside the file, is not drawn and costs nothing.
    fn sized(&self, image: &gltf::Image<'_>) -> Option<Picture> {
        let bytes = self.picture(image)?;
        let size = imagesize::blob_size(bytes)
            .map_err(|e| log::warn!("picture {}: {e}", image.index()))
            .ok()?;
        Some(Picture {
            bytes: Vec::new(),
            width: u32::try_from(size.width).ok()?,
            height: u32::try_from(size.height).ok()?,
        })
    }
}

/// The picture of a material's base colour, if it has one.
fn base_image<'a>(material: &gltf::Material<'a>) -> Option<gltf::Image<'a>> {
    let info = material.pbr_metallic_roughness().base_color_texture()?;
    Some(info.texture().source())
}

impl Model {
    /// Every mesh the scene of a GLB places, at rest: a mesh where its node
    /// stands, a skinned one where its joints stand. Only triangles are
    /// read; a primitive of points or lines, or one whose indices reach
    /// past its vertices, is left out.
    pub fn read(glb: &Glb) -> Result<Model, Error> {
        let hierarchy = Hierarchy::read(&glb.document);
        let world = hierarchy.world(&hierarchy.rest);
        let mut model = Model::default();
        // A glTF material's index here, the plain one last, and a glTF
        // image's: each made once, when first drawn.
        let mut materials: Vec<Option<usize>> = vec![None; glb.document.materials().len() + 1];
        let mut pictures: Vec<Option<Option<usize>>> = vec![None; glb.document.images().len()];
        let mut runs: Vec<Vec<u32>> = Vec::new();

        for node in glb.placed(&hierarchy) {
            let Some(mesh) = node.mesh() else {
                continue;
            };
            let joints = node.skin().map(|skin| {
                let inverse_bind: Vec<Mat4> = skin
                    .reader(|buffer| glb.buffer(buffer))
                    .read_inverse_bind_matrices()
                    .map(|matrices| matrices.map(|m| Mat4::from_cols_array_2d(&m)).collect())
                    .unwrap_or_default();
                skin.joints()
                    .enumerate()
                    .map(|(k, joint)| {
                        world[joint.index()] * inverse_bind.get(k).copied().unwrap_or_default()
                    })
                    .collect::<Vec<Mat4>>()
            });
            for primitive in mesh.primitives() {
                if primitive.mode() != Mode::Triangles {
                    continue;
                }
                let gltf_material = primitive.material();
                let slot = gltf_material.index().unwrap_or(materials.len() - 1);
                let material = *materials[slot].get_or_insert_with(|| {
                    let picture = base_image(&gltf_material).and_then(|image| {
                        *pictures[image.index()].get_or_insert_with(|| {
                            let mut picture = glb.sized(&image)?;
                            picture.bytes = glb.picture(&image)?.to_vec();
                            model.pictures.push(picture);
                            Some(model.pictures.len() - 1)
                        })
                    });
                    model.materials.push(match gltf_material.index() {
                        Some(_) => material_of(&gltf_material, picture),
                        None => PLAIN,
                    });
                    runs.push(Vec::new());
                    model.materials.len() - 1
                });
                let set = gltf_material
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map_or(0, |info| info.tex_coord());
                let place = match &joints {
                    Some(joints) => Place::Joints(joints),
                    None => Place::Node(world[node.index()]),
                };
                read_primitive(
                    glb,
                    &primitive,
                    set,
                    place,
                    &mut model.vertices,
                    &mut runs[material],
                );
            }
        }

        for (material, indices) in runs.into_iter().enumerate() {
            if indices.is_empty() {
                continue;
            }
            let start = model.indices.len() as u32;
            model.indices.extend(indices);
            model.runs.push(Run {
                indices: start..model.indices.len() as u32,
                material,
            });
        }
        if model.runs.is_empty() {
            return Err(Error("no triangles".into()));
        }
        let (min, max) = model.vertices.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(min, max), vertex| {
                let at = Vec3::from(vertex.position);
                (min.min(at), max.max(at))
            },
        );
        model.min = min.into();
        model.max = max.into();
        Ok(model)
    }
}

fn material_of(material: &gltf::Material<'_>, picture: Option<usize>) -> Material {
    Material {
        color: material.pbr_metallic_roughness().base_color_factor(),
        picture,
        alpha: match material.alpha_mode() {
            gltf::material::AlphaMode::Opaque => Alpha::Opaque,
            gltf::material::AlphaMode::Mask => Alpha::Mask(material.alpha_cutoff().unwrap_or(0.5)),
            gltf::material::AlphaMode::Blend => Alpha::Blend,
        },
        both_sides: material.double_sided(),
    }
}

/// An attribute read whole: one a vertex, or none.
fn complete<T>(values: Option<impl Iterator<Item = T>>, count: usize) -> Option<Vec<T>> {
    values
        .map(Iterator::collect::<Vec<T>>)
        .filter(|values| values.len() == count)
}

/// Where a primitive's vertices stand.
#[derive(Clone, Copy)]
enum Place<'a> {
    /// Where its node stands.
    Node(Mat4),
    /// Where its joints stand at rest, each joint's matrix from the mesh's
    /// space as its skin binds it.
    Joints(&'a [Mat4]),
}

/// One primitive's triangles, placed: its vertices appended to `vertices`
/// and its triangles to `indices`. With no normals said, each triangle gets
/// its own corners and its own flat normal, as glTF asks.
fn read_primitive(
    glb: &Glb,
    primitive: &gltf::Primitive<'_>,
    set: u32,
    place: Place<'_>,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
) {
    let reader = primitive.reader(|buffer| glb.buffer(buffer));
    let Some(positions) = reader.read_positions() else {
        return;
    };
    let positions: Vec<[f32; 3]> = positions.collect();
    let count = positions.len();
    let corners: Vec<u32> = match reader.read_indices() {
        Some(read) => read.into_u32().collect(),
        None => (0..count as u32).collect(),
    };
    let corners = &corners[..corners.len() / 3 * 3];
    if corners.iter().any(|&i| i as usize >= count) {
        log::warn!("a primitive whose indices reach past its vertices is left out");
        return;
    }
    let normals = complete(reader.read_normals(), count);
    let uvs = complete(reader.read_tex_coords(set).map(|uv| uv.into_f32()), count);
    let colors = complete(
        reader.read_colors(0).map(|color| color.into_rgba_f32()),
        count,
    );
    let skin = match place {
        Place::Joints(_) => complete(
            reader
                .read_joints(0)
                .zip(reader.read_weights(0))
                .map(|(joints, weights)| joints.into_u16().zip(weights.into_f32())),
            count,
        ),
        Place::Node(_) => None,
    };

    // Each vertex's matrix: one for a mesh where its node stands, and for a
    // skinned one its joints' blended by its weights.
    let (node, blended): (Mat4, Option<Vec<Mat4>>) = match (place, skin) {
        (Place::Node(matrix), _) => (matrix, None),
        (Place::Joints(joints), Some(skin)) => {
            let blend = |(which, weights): ([u16; 4], [f32; 4])| {
                let sum: f32 = weights.iter().sum();
                let share = if sum > 0.0 { sum } else { 1.0 };
                which
                    .into_iter()
                    .zip(weights)
                    .filter_map(|(joint, weight)| {
                        Some(*joints.get(usize::from(joint))? * (weight / share))
                    })
                    .fold(Mat4::ZERO, |matrix, joint| matrix + joint)
            };
            (Mat4::IDENTITY, Some(skin.into_iter().map(blend).collect()))
        }
        // A skin with no joints or weights read: its mesh as it is.
        (Place::Joints(_), None) => (Mat4::IDENTITY, None),
    };
    let matrix = |i: usize| blended.as_ref().map_or(node, |blended| blended[i]);
    let placed: Vec<Vertex> = (0..count)
        .map(|i| {
            let m = matrix(i);
            let normal = normals.as_ref().map_or(Vec3::Y, |n| Vec3::from(n[i]));
            let turn = Mat3::from_mat4(m);
            let normal = match turn.determinant().abs() > f32::EPSILON {
                true => turn.inverse().transpose() * normal,
                false => turn * normal,
            };
            Vertex {
                position: m.transform_point3(Vec3::from(positions[i])).into(),
                normal: normal.normalize_or(Vec3::Y).into(),
                uv: uvs.as_ref().map_or([0.0; 2], |uv| uv[i]),
                color: colors.as_ref().map_or([1.0; 4], |c| c[i]),
            }
        })
        .collect();
    // A node that mirrors turns its triangles inside out: wound back.
    let mirrored = |i: u32| Mat3::from_mat4(matrix(i as usize)).determinant() < 0.0;

    match normals {
        Some(_) => {
            let base = vertices.len() as u32;
            vertices.extend(placed);
            for triangle in corners.chunks_exact(3) {
                let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
                let (b, c) = if mirrored(a) { (c, b) } else { (b, c) };
                indices.extend([base + a, base + b, base + c]);
            }
        }
        None => {
            for triangle in corners.chunks_exact(3) {
                let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
                let (b, c) = if mirrored(a) { (c, b) } else { (b, c) };
                let [pa, pb, pc] = [a, b, c].map(|i| Vec3::from(placed[i as usize].position));
                let normal = (pb - pa).cross(pc - pa).normalize_or(Vec3::Y);
                let base = vertices.len() as u32;
                for i in [a, b, c] {
                    vertices.push(Vertex {
                        normal: normal.into(),
                        ..placed[i as usize]
                    });
                }
                indices.extend([base, base + 1, base + 2]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GLB of this JSON and this binary chunk, each padded as glTF asks.
    fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut json = json.as_bytes().to_vec();
        json.resize(json.len().next_multiple_of(4), b' ');
        let mut bin = bin.to_vec();
        bin.resize(bin.len().next_multiple_of(4), 0);
        let total = (12 + 8 + json.len() + 8 + bin.len()) as u32;
        let mut out = Vec::new();
        out.extend(b"glTF");
        out.extend(2u32.to_le_bytes());
        out.extend(total.to_le_bytes());
        out.extend((json.len() as u32).to_le_bytes());
        out.extend(b"JSON");
        out.extend(json);
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(bin);
        out
    }

    /// One triangle, counter clockwise seen from `+z`, and the header of a
    /// PNG of 2048 by 1024 texels: its size is all a weight reads of it.
    fn triangle_and_picture() -> Vec<u8> {
        let mut bin = Vec::new();
        for corner in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for value in corner {
                bin.extend(value.to_le_bytes());
            }
        }
        bin.extend([0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        bin.extend(13u32.to_be_bytes());
        bin.extend(b"IHDR");
        bin.extend(2048u32.to_be_bytes());
        bin.extend(1024u32.to_be_bytes());
        bin.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
        bin
    }

    const PLACED_THRICE: &str = r#"{
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0, 1, 2]}],
        "nodes": [
            {"mesh": 0},
            {"mesh": 0, "translation": [2, 0, 0]},
            {"mesh": 0, "translation": [-2, 0, 0], "scale": [-1, 1, 1]},
            {"mesh": 0, "translation": [50, 0, 0]}
        ],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "material": 0}]}],
        "materials": [{
            "pbrMetallicRoughness": {
                "baseColorTexture": {"index": 0},
                "baseColorFactor": [1, 0.5, 0.25, 1]
            },
            "doubleSided": true,
            "alphaMode": "MASK",
            "alphaCutoff": 0.25
        }],
        "textures": [{"source": 0}],
        "images": [{"bufferView": 1, "mimeType": "image/png"}],
        "accessors": [{
            "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
            "min": [0, 0, 0], "max": [1, 1, 0]
        }],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": 36},
            {"buffer": 0, "byteOffset": 36, "byteLength": 33}
        ],
        "buffers": [{"byteLength": 72}]
    }"#;

    #[test]
    fn a_scene_is_read_where_its_nodes_stand() {
        let glb = Glb::open(&glb(PLACED_THRICE, &triangle_and_picture())).unwrap();
        let model = Model::read(&glb).unwrap();
        // Three triangles, the fourth node being in no scene: no normals
        // said, so each has its own corners.
        assert_eq!(model.indices.len(), 9);
        assert_eq!(model.vertices.len(), 9);
        assert_eq!(
            model.runs,
            vec![Run {
                indices: 0..9,
                material: 0
            }]
        );
        assert_eq!(model.min, [-3.0, 0.0, 0.0]);
        assert_eq!(model.max, [3.0, 1.0, 0.0]);
        // The mirrored one too faces `+z`, wound counter clockwise.
        for triangle in model.indices.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|k| model.vertices[triangle[k] as usize]);
            let wound = (Vec3::from(b.position) - Vec3::from(a.position))
                .cross(Vec3::from(c.position) - Vec3::from(a.position));
            assert!(wound.z > 0.0, "{triangle:?}");
            assert_eq!(a.normal, [0.0, 0.0, 1.0]);
        }
        assert_eq!(
            model.materials,
            vec![Material {
                color: [1.0, 0.5, 0.25, 1.0],
                picture: Some(0),
                alpha: Alpha::Mask(0.25),
                both_sides: true,
            }]
        );
        let picture = &model.pictures[0];
        assert_eq!((picture.width, picture.height), (2048, 1024));
        assert_eq!(picture.bytes.len(), 33);
    }

    #[test]
    fn a_glb_weighs_what_it_draws_before_it_is_read() {
        let glb = Glb::open(&glb(PLACED_THRICE, &triangle_and_picture())).unwrap();
        assert_eq!(
            glb.weigh(),
            Weight {
                triangles: 3,
                both_sides: 3,
                texels: 2048 * 1024,
                materials: 1,
            }
        );
    }

    #[test]
    fn an_index_that_names_nothing_is_refused() {
        let astray = PLACED_THRICE.replace(
            r#"{"mesh": 0, "translation": [50"#,
            r#"{"mesh": 7, "translation": [50"#,
        );
        assert!(Glb::open(&glb(&astray, &triangle_and_picture())).is_err());
    }

    #[test]
    fn children_that_loop_back_are_walked_once() {
        let looped = PLACED_THRICE
            .replace(r#""nodes": [0, 1, 2]}"#, r#""nodes": [0]}"#)
            .replace(r#"{"mesh": 0},"#, r#"{"children": [1]},"#)
            .replace(
                r#"{"mesh": 0, "translation": [2, 0, 0]}"#,
                r#"{"mesh": 0, "children": [0]}"#,
            );
        let glb = Glb::open(&glb(&looped, &triangle_and_picture())).unwrap();
        assert_eq!(glb.weigh().triangles, 1);
        assert_eq!(Model::read(&glb).unwrap().indices.len(), 3);
    }
}
