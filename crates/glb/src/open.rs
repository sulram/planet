//! Opening a GLB, and its node hierarchy at rest.

use glam::{Mat4, Quat, Vec3};

use crate::Error;

pub struct Glb {
    pub document: gltf::Document,
    pub blob: Vec<u8>,
}

impl Glb {
    /// Opens a binary glTF. Of glTF's rules, only that every index names
    /// something is held, since the readers stop on one that does not:
    /// exporters break small rules that do not matter to what is read, and
    /// what is read is checked where it is read.
    pub fn open(bytes: &[u8]) -> Result<Glb, Error> {
        use gltf::json::validation::{Error as Rule, Validate};
        let gltf =
            gltf::Gltf::from_slice_without_validation(bytes).map_err(|e| Error(e.to_string()))?;
        let root = gltf.document.as_json();
        let mut astray = None;
        root.validate(root, gltf::json::Path::new, &mut |path, rule| {
            if rule == Rule::IndexOutOfBounds && astray.is_none() {
                astray = Some(path().to_string());
            }
        });
        if let Some(path) = astray {
            return Err(Error(format!("{path} names nothing")));
        }
        let blob = gltf.blob.ok_or_else(|| Error("no binary chunk".into()))?;
        Ok(Glb {
            document: gltf.document,
            blob,
        })
    }

    /// Buffer access for the `gltf` readers: a GLB has one buffer, the blob.
    pub fn buffer(&self, buffer: gltf::Buffer<'_>) -> Option<&[u8]> {
        (buffer.index() == 0).then_some(self.blob.as_slice())
    }

    /// The picture a texture shows: its source, else the WebP that
    /// `EXT_texture_webp` names in its place. `None` for one whose picture
    /// is in a form never read, such as a KTX2 of `KHR_texture_basisu`.
    pub fn source<'a>(&'a self, texture: &gltf::Texture<'a>) -> Option<gltf::Image<'a>> {
        texture.source().or_else(|| {
            let webp = texture.extensions()?.get("EXT_texture_webp")?;
            let index = usize::try_from(webp.get("source")?.as_u64()?).ok()?;
            self.document.images().nth(index)
        })
    }

    /// The bytes of a picture the file holds, as they were written. `None`
    /// for a picture outside the file, which is never fetched.
    pub fn picture(&self, image: &gltf::Image<'_>) -> Option<&[u8]> {
        let gltf::image::Source::View { view, .. } = image.source() else {
            return None;
        };
        self.buffer(view.buffer())?
            .get(view.offset()..view.offset().checked_add(view.length())?)
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
    /// Node indices, every parent before its children, each once.
    pub order: Vec<usize>,
}

impl Hierarchy {
    pub fn read(document: &gltf::Document) -> Hierarchy {
        let nodes: Vec<gltf::Node<'_>> = document.nodes().collect();
        let mut parent = vec![None; nodes.len()];
        let mut rest = Vec::with_capacity(nodes.len());
        for node in &nodes {
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
        // A node is walked once: a file whose children loop back is walked
        // to the end of the loop, never round it.
        let mut seen = vec![false; nodes.len()];
        let mut order = Vec::with_capacity(nodes.len());
        let mut stack: Vec<usize> = (0..nodes.len()).filter(|&i| parent[i].is_none()).collect();
        while let Some(index) = stack.pop() {
            if std::mem::replace(&mut seen[index], true) {
                continue;
            }
            order.push(index);
            stack.extend(nodes[index].children().map(|child| child.index()));
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
