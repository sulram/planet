//! What avatars and clips share: opening a GLB and its node hierarchy at rest.

use glam::{Mat4, Quat, Vec3};

use crate::Error;

pub struct Glb {
    pub document: gltf::Document,
    pub blob: Vec<u8>,
}

impl Glb {
    /// Opens a binary glTF. Validation is skipped: VRM exporters break small
    /// rules that do not matter to what we read.
    pub fn open(bytes: &[u8]) -> Result<Glb, Error> {
        let gltf = gltf::Gltf::from_slice_without_validation(bytes)
            .map_err(|e| Error::Gltf(e.to_string()))?;
        let blob = gltf
            .blob
            .ok_or_else(|| Error::Gltf("no binary chunk".into()))?;
        Ok(Glb {
            document: gltf.document,
            blob,
        })
    }

    /// Buffer access for the `gltf` readers: a GLB has one buffer, the blob.
    pub fn buffer(&self, buffer: gltf::Buffer<'_>) -> Option<&[u8]> {
        (buffer.index() == 0).then_some(self.blob.as_slice())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

/// The nodes of a document at rest, with parents resolved.
pub struct Hierarchy {
    pub rest: Vec<Transform>,
    pub parent: Vec<Option<usize>>,
    /// Node indices, every parent before its children.
    pub order: Vec<usize>,
}

impl Hierarchy {
    pub fn read(document: &gltf::Document) -> Hierarchy {
        let count = document.nodes().len();
        let mut parent = vec![None; count];
        let mut rest = Vec::with_capacity(count);
        for node in document.nodes() {
            let (translation, rotation, scale) = node.transform().decomposed();
            rest.push(Transform {
                translation: Vec3::from(translation),
                rotation: Quat::from_array(rotation).normalize(),
                scale: Vec3::from(scale),
            });
            for child in node.children() {
                parent[child.index()] = Some(node.index());
            }
        }
        let mut order = Vec::with_capacity(count);
        let mut stack: Vec<usize> = (0..count).filter(|&i| parent[i].is_none()).collect();
        while let Some(index) = stack.pop() {
            order.push(index);
            let node = document.nodes().nth(index).expect("a node index");
            stack.extend(node.children().map(|child| child.index()));
        }
        Hierarchy {
            rest,
            parent,
            order,
        }
    }

    /// World matrices for the given local transforms.
    pub fn world(&self, local: &[Transform]) -> Vec<Mat4> {
        let mut world = vec![Mat4::IDENTITY; local.len()];
        for &index in &self.order {
            let matrix = local[index].matrix();
            world[index] = match self.parent[index] {
                Some(parent) => world[parent] * matrix,
                None => matrix,
            };
        }
        world
    }
}
