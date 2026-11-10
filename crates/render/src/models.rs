//! Model meshes: the cubes of parts and the meshes of 3D models, each in
//! its own space and placed by one matrix a frame, relative to the camera,
//! as an avatar is. Cubes are drawn with the volumes' own sides
//! (`shaders/cubes.wgsl`), so a part is lit, edged and glazed as the cells
//! are; a 3D model with its pictures (`shaders/textured.wgsl`), a run of
//! triangles a material, each picture with its mip chain.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{DAffine3, DMat4, DVec3, Vec3};
use scene::{
    Image, ModelChange, ModelDraw, ModelId, ModelPass, TexturedMesh, TexturedRun, TexturedVertex,
    VolumeVertex,
};
use wgpu::util::DeviceExt;

use crate::mips::{FORMAT, Mips, fit};
use crate::{PipelineSpec, Surface, pipeline};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceUniform {
    relative_from_model: [[f32; 4]; 4],
    normal_from_model: [[f32; 4]; 4],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct MaterialUniform {
    /// `x`: the alpha under which a pixel is not drawn.
    cutoff: [f32; 4],
}

struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    /// How far a corner reaches from the mesh's anchor, in its own units.
    reach: f32,
    /// The runs of a mesh drawn with pictures, each with its material;
    /// empty for cubes.
    runs: Vec<(Range<u32>, wgpu::BindGroup)>,
}

impl Mesh {
    fn textured(&self) -> bool {
        !self.runs.is_empty()
    }
}

pub struct Models {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
    over_pipeline: wgpu::RenderPipeline,
    textured_pipeline: wgpu::RenderPipeline,
    textured_shadow_pipeline: wgpu::RenderPipeline,
    instance_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// The picture of a run that has none: white, so its corners' colour
    /// is all it shows.
    white: wgpu::TextureView,
    mips: Mips,
    /// The widest texture the device takes on a side.
    max_side: u32,
    meshes: HashMap<ModelId, Mesh>,
}

/// The sides of cubes with the vertex stage of a model in front.
fn source() -> String {
    format!(
        "{}\n{}",
        include_str!("shaders/cubes.wgsl"),
        include_str!("shaders/model.wgsl")
    )
}

impl Models {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
        mips: Mips,
    ) -> Models {
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model instance"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let buffers = &[Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<VolumeVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Unorm8x4
            ],
        })];
        let source = source();
        let make = |layout: &wgpu::BindGroupLayout, surface: Surface| {
            pipeline(
                device,
                format,
                PipelineSpec {
                    label: "model",
                    source: &source,
                    layouts: &[layout, &instance_layout],
                    buffers,
                    surface,
                },
            )
        };
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model material"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let textured_buffers = &[Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<TexturedVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Unorm8x4
            ],
        })];
        let textured = |layout: &wgpu::BindGroupLayout, surface: Surface| {
            pipeline(
                device,
                format,
                PipelineSpec {
                    label: "textured model",
                    source: include_str!("shaders/textured.wgsl"),
                    layouts: &[layout, &instance_layout, &material_layout],
                    buffers: textured_buffers,
                    surface,
                },
            )
        };
        // A model's pictures repeat: a glTF's coordinates may run past one.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model picture"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        let white = device
            .create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("white"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &[255; 4],
            )
            .create_view(&wgpu::TextureViewDescriptor::default());
        Models {
            pipeline: make(view_layout, Surface::Solid),
            shadow_pipeline: make(shadow_layout, Surface::Shadow),
            glass_pipeline: make(view_layout, Surface::Glass),
            over_pipeline: make(view_layout, Surface::Overlay),
            textured_pipeline: textured(view_layout, Surface::Solid),
            textured_shadow_pipeline: textured(shadow_layout, Surface::ShadowCutout),
            instance_layout,
            material_layout,
            sampler,
            white,
            mips,
            max_side: device.limits().max_texture_dimension_2d,
            meshes: HashMap::new(),
        }
    }

    pub fn apply(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, change: ModelChange) {
        match change {
            ModelChange::Add(id, mesh) if !mesh.indices.is_empty() => {
                let positions = mesh.vertices.iter().map(|v| v.position);
                let mesh = Mesh::new(device, &mesh.vertices, &mesh.indices, positions, Vec::new());
                self.meshes.insert(id, mesh);
            }
            ModelChange::Textured(id, mesh) if !mesh.runs.is_empty() => {
                let TexturedMesh {
                    vertices,
                    indices,
                    runs,
                    images,
                } = mesh;
                let runs = self.materials(device, queue, &runs, images);
                let positions = vertices.iter().map(|v| v.position);
                let mesh = Mesh::new(device, &vertices, &indices, positions, runs);
                self.meshes.insert(id, mesh);
            }
            ModelChange::Add(id, _) | ModelChange::Textured(id, _) | ModelChange::Remove(id) => {
                self.meshes.remove(&id);
            }
        }
    }

    /// Each run of a mesh drawn with pictures, with its material: its
    /// picture uploaded once with its mip chain, however many runs show it.
    fn materials(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        runs: &[TexturedRun],
        images: Vec<Image>,
    ) -> Vec<(Range<u32>, wgpu::BindGroup)> {
        let pictures: Vec<wgpu::TextureView> = images
            .into_iter()
            .map(|image| {
                let image = fit(&Arc::new(image), self.max_side);
                let texture = self
                    .mips
                    .create(device, "model picture", image.width, image.height);
                self.mips.write(queue, &texture, &image);
                self.mips.generate(device, queue, &texture);
                texture.create_view(&wgpu::TextureViewDescriptor::default())
            })
            .collect();
        runs.iter()
            .map(|run| {
                let picture = run
                    .image
                    .and_then(|image| pictures.get(image))
                    .unwrap_or(&self.white);
                let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model material"),
                    contents: bytemuck::bytes_of(&MaterialUniform {
                        cutoff: [run.cutoff, 0.0, 0.0, 0.0],
                    }),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("model material"),
                    layout: &self.material_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(picture),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: uniform.as_entire_binding(),
                        },
                    ],
                });
                (run.indices.clone(), group)
            })
            .collect()
    }

    /// Draws the solid models [`InstanceUniforms::write`] placed.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &InstanceUniforms,
        placed: &Placed,
    ) {
        let solid: Vec<_> = placed
            .solid
            .iter()
            .map(|&(slot, id, _, _)| (slot, id))
            .collect();
        self.draw_solid(
            pass,
            uniforms,
            &solid,
            [&self.pipeline, &self.textured_pipeline],
        );
    }

    /// Solid models, the cubes and then the meshes drawn with pictures,
    /// each kind with its pipeline.
    fn draw_solid(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &InstanceUniforms,
        drawn: &[(usize, ModelId)],
        [cubes, textured]: [&wgpu::RenderPipeline; 2],
    ) {
        for (pipeline, kind) in [(cubes, false), (textured, true)] {
            let mut set = false;
            for &(slot, id) in drawn {
                if self.meshes[&id].textured() != kind {
                    continue;
                }
                if !std::mem::replace(&mut set, true) {
                    pass.set_pipeline(pipeline);
                }
                self.draw_one(pass, uniforms, slot, id);
            }
        }
    }

    pub fn draw_shadow(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &InstanceUniforms,
        placed: &Placed,
        matrix: glam::Mat4,
    ) {
        let casting: Vec<_> = placed
            .solid
            .iter()
            .filter(|&&(_, _, center, radius)| crate::shadow::intersects(matrix, center, radius))
            .map(|&(slot, id, _, _)| (slot, id))
            .collect();
        self.draw_solid(
            pass,
            uniforms,
            &casting,
            [&self.shadow_pipeline, &self.textured_shadow_pipeline],
        );
    }

    /// The glass, after everything opaque, the sea and the clouds: it tests
    /// depth and writes none. Then the marks over everything.
    pub fn draw_over(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &InstanceUniforms,
        placed: &Placed,
    ) {
        let passes = [
            (&self.glass_pipeline, &placed.glass),
            (&self.over_pipeline, &placed.over),
        ];
        for (pipeline, drawn) in passes {
            if drawn.is_empty() {
                continue;
            }
            pass.set_pipeline(pipeline);
            // Glass and marks are cubes: a 3D model is always solid.
            for &(slot, id) in drawn {
                if !self.meshes[&id].textured() {
                    self.draw_one(pass, uniforms, slot, id);
                }
            }
        }
    }

    fn draw_one(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &InstanceUniforms,
        slot: usize,
        id: ModelId,
    ) {
        let mesh = &self.meshes[&id];
        pass.set_bind_group(1, &uniforms.slots[slot].1, &[]);
        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        if mesh.runs.is_empty() {
            pass.draw_indexed(0..mesh.count, 0, 0..1);
        }
        for (indices, material) in &mesh.runs {
            pass.set_bind_group(2, material, &[]);
            pass.draw_indexed(indices.clone(), 0, 0..1);
        }
    }
}

impl Mesh {
    fn new<V: Pod>(
        device: &wgpu::Device,
        vertices: &[V],
        indices: &[u32],
        positions: impl Iterator<Item = [f32; 3]>,
        runs: Vec<(Range<u32>, wgpu::BindGroup)>,
    ) -> Mesh {
        let reach = positions
            .map(|position| Vec3::from(position).length())
            .fold(0.0, f32::max);
        Mesh {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("model"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("model indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            count: indices.len() as u32,
            reach,
            runs,
        }
    }
}

/// What one view draws of the models this frame: each by its slot and
/// mesh, a solid one with the centre of its ball relative to the camera
/// and its radius, for the shadow passes to skip what casts nothing in them.
#[derive(Default)]
pub struct Placed {
    solid: Vec<(usize, ModelId, Vec3, f32)>,
    glass: Vec<(usize, ModelId)>,
    over: Vec<(usize, ModelId)>,
}

impl Placed {
    /// Whether anything is drawn over the picture once the sea and the
    /// clouds are in it.
    pub fn over(&self) -> bool {
        !self.glass.is_empty() || !self.over.is_empty()
    }
}

/// One uniform block per model a view draws. Grows, never shrinks.
#[derive(Default)]
pub struct InstanceUniforms {
    slots: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
}

impl InstanceUniforms {
    /// Writes the models whose mesh the renderer holds, relative to the
    /// camera, and returns them in slot order. A mesh named before it was
    /// applied is skipped, never a panic.
    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        models: &Models,
        drawn: &[ModelDraw],
        camera: DVec3,
    ) -> Placed {
        let mut placed = Placed::default();
        let held = drawn
            .iter()
            .filter(|draw| models.meshes.contains_key(&draw.mesh));
        for (slot, draw) in held.enumerate() {
            if slot == self.slots.len() {
                self.slots
                    .push(Self::allocate(device, &models.instance_layout));
            }
            // Subtract the camera in f64, only then drop to f32.
            let relative = DAffine3::from_translation(-camera) * draw.transform;
            let normal = relative.matrix3.inverse().transpose();
            let uniform = InstanceUniform {
                relative_from_model: DMat4::from(relative).as_mat4().to_cols_array_2d(),
                normal_from_model: DMat4::from_mat3(normal).as_mat4().to_cols_array_2d(),
            };
            queue.write_buffer(&self.slots[slot].0, 0, bytemuck::bytes_of(&uniform));
            match draw.pass {
                ModelPass::Glass => placed.glass.push((slot, draw.mesh)),
                ModelPass::Over => placed.over.push((slot, draw.mesh)),
                ModelPass::Solid => {
                    let mesh = &models.meshes[&draw.mesh];
                    let stretch = [relative.x_axis, relative.y_axis, relative.z_axis]
                        .map(DVec3::length)
                        .into_iter()
                        .fold(0.0, f64::max);
                    let center = relative.translation.as_vec3();
                    placed
                        .solid
                        .push((slot, draw.mesh, center, mesh.reach * stretch as f32));
                }
            }
        }
        placed
    }

    fn allocate(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model instance"),
            size: size_of::<InstanceUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model instance"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, bind_group)
    }
}
