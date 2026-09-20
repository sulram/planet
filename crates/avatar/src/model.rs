//! A VRM 0.x avatar: skinned mesh, skeleton, humanoid map.

use std::sync::Arc;

use glam::Mat4;
use scene::{Image, MAX_JOINTS, SkinnedMesh, SkinnedPrimitive, SkinnedVertex};

use crate::Error;
use crate::bones::{self, BONES, HIPS};
use crate::clip::Pose;
use crate::glb::{Glb, Hierarchy};

pub struct Avatar {
    /// Shared with the renderer; never changes after load.
    pub mesh: Arc<SkinnedMesh>,
    hierarchy: Hierarchy,
    /// Node of each skin joint, in skin order.
    joints: Vec<usize>,
    inverse_bind: Vec<Mat4>,
    /// Node of each humanoid bone, by index in [`BONES`].
    humanoid: Vec<Option<usize>>,
    /// Height of the hips above the feet at rest, metres: the scale of the
    /// hips motion in clips.
    hips_height_m: f32,
}

impl Avatar {
    pub fn from_vrm(bytes: &[u8]) -> Result<Avatar, Error> {
        let glb = Glb::open(bytes)?;
        let humanoid = humanoid(&glb.document)?;
        let hierarchy = Hierarchy::read(&glb.document);

        let skin = glb
            .document
            .skins()
            .next()
            .ok_or_else(|| Error::NotVrm("no skin".into()))?;
        let joints: Vec<usize> = skin.joints().map(|node| node.index()).collect();
        if joints.len() > MAX_JOINTS {
            return Err(Error::NotVrm(format!(
                "{} joints, the limit is {MAX_JOINTS}",
                joints.len()
            )));
        }
        let inverse_bind: Vec<Mat4> = skin
            .reader(|buffer| glb.buffer(buffer))
            .read_inverse_bind_matrices()
            .map(|matrices| matrices.map(|m| Mat4::from_cols_array_2d(&m)).collect())
            .unwrap_or_else(|| vec![Mat4::IDENTITY; joints.len()]);

        let hips = humanoid[HIPS].ok_or_else(|| Error::NotVrm("no hips bone".into()))?;
        let hips_height_m = hierarchy.world(&hierarchy.rest)[hips].w_axis.y;

        let mesh = Arc::new(mesh(&glb, skin.index())?);
        Ok(Avatar {
            mesh,
            hierarchy,
            joints,
            inverse_bind,
            humanoid,
            hips_height_m,
        })
    }

    /// Joint matrices (mesh space from rest space) for a pose.
    pub fn joint_matrices(&self, pose: &Pose) -> Vec<Mat4> {
        let mut local = self.hierarchy.rest.clone();
        for (bone, node) in self.humanoid.iter().enumerate() {
            let (Some(node), Some(rotation)) = (node, pose.rotations.get(bone).copied().flatten())
            else {
                continue;
            };
            // VRM 0.x bones rest with no rotation, so the pose is the local
            // rotation. Composing keeps odd exports working too.
            local[*node].rotation = self.hierarchy.rest[*node].rotation * rotation;
        }
        if let (Some(hips), Some(offset)) = (self.humanoid[HIPS], pose.hips) {
            local[hips].translation = offset * self.hips_height_m;
        }
        let world = self.hierarchy.world(&local);
        self.joints
            .iter()
            .zip(&self.inverse_bind)
            .map(|(&node, bind)| world[node] * *bind)
            .collect()
    }
}

/// Reads `extensions.VRM.humanoid.humanBones`.
fn humanoid(document: &gltf::Document) -> Result<Vec<Option<usize>>, Error> {
    let vrm = document
        .extensions()
        .and_then(|extensions| extensions.get("VRM"))
        .ok_or_else(|| Error::NotVrm("no VRM 0.x extension".into()))?;
    let listed = vrm["humanoid"]["humanBones"]
        .as_array()
        .ok_or_else(|| Error::NotVrm("no humanoid bones".into()))?;
    let mut map = vec![None; BONES.len()];
    for entry in listed {
        let (Some(name), Some(node)) = (entry["bone"].as_str(), entry["node"].as_u64()) else {
            continue;
        };
        if let Some(bone) = bones::from_vrm(name) {
            map[bone] = Some(node as usize);
        }
    }
    Ok(map)
}

/// Every primitive skinned by `skin`, merged into one vertex and index buffer.
fn mesh(glb: &Glb, skin: usize) -> Result<SkinnedMesh, Error> {
    let mut out = SkinnedMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        primitives: Vec::new(),
        images: Vec::new(),
    };
    // glTF image index -> index in `out.images`, decoded once.
    let mut decoded: Vec<Option<Option<usize>>> = vec![None; glb.document.images().len()];

    let skinned = glb
        .document
        .nodes()
        .filter(|node| node.skin().is_some_and(|s| s.index() == skin));
    for primitive in skinned
        .filter_map(|node| node.mesh())
        .flat_map(|mesh| mesh.primitives())
    {
        let reader = primitive.reader(|buffer| glb.buffer(buffer));
        let (Some(positions), Some(joints), Some(weights)) = (
            reader.read_positions(),
            reader.read_joints(0),
            reader.read_weights(0),
        ) else {
            log::warn!("skipping a primitive without positions, joints or weights");
            continue;
        };
        let positions: Vec<[f32; 3]> = positions.collect();
        let mut normals = reader.read_normals();
        let mut uvs = reader.read_tex_coords(0).map(|uv| uv.into_f32());
        let base = out.vertices.len() as u32;
        for ((position, joints), weights) in positions
            .iter()
            .zip(joints.into_u16())
            .zip(weights.into_f32())
        {
            // Exporters do not always normalise weights; skinning assumes it.
            let sum: f32 = weights.iter().sum();
            let weights = if sum > 0.0 {
                weights.map(|w| w / sum)
            } else {
                [1.0, 0.0, 0.0, 0.0]
            };
            out.vertices.push(SkinnedVertex {
                position: *position,
                normal: normals
                    .as_mut()
                    .and_then(Iterator::next)
                    .unwrap_or([0.0, 1.0, 0.0]),
                uv: uvs.as_mut().and_then(Iterator::next).unwrap_or([0.0, 0.0]),
                joints,
                weights,
            });
        }

        let start = out.indices.len() as u32;
        match reader.read_indices() {
            Some(indices) => out.indices.extend(indices.into_u32().map(|i| base + i)),
            None => out.indices.extend(base..base + positions.len() as u32),
        }
        let texture = primitive
            .material()
            .pbr_metallic_roughness()
            .base_color_texture();
        let image = texture.and_then(|info| {
            let index = info.texture().source().index();
            *decoded[index].get_or_insert_with(|| {
                let image = image(glb, glb.document.images().nth(index)?)?;
                out.images.push(image);
                Some(out.images.len() - 1)
            })
        });
        out.primitives.push(SkinnedPrimitive {
            indices: start..out.indices.len() as u32,
            image,
        });
    }
    if out.primitives.is_empty() {
        return Err(Error::NotVrm("no skinned geometry".into()));
    }
    Ok(out)
}

/// Decodes an embedded PNG to RGBA8. Anything else draws white, with a log.
fn image(glb: &Glb, image: gltf::Image<'_>) -> Option<Image> {
    let gltf::image::Source::View { view, .. } = image.source() else {
        log::warn!("external images are not loaded");
        return None;
    };
    let bytes = glb.blob.get(view.offset()..view.offset() + view.length())?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| log::warn!("texture is not a PNG: {e}"))
        .ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut pixels).ok()?;
    pixels.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}
