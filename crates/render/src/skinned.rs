//! Skinned meshes: avatars. Skinning runs in the vertex shader from a uniform
//! block of joint matrices, which is what WebGL2 can do too.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::{DAffine3, DMat4, DVec3};
use scene::{Image, MAX_JOINTS, SkinnedChange, SkinnedInstance, SkinnedMeshId, SkinnedVertex};
use wgpu::util::DeviceExt;

use crate::{PipelineSpec, pipeline};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceUniform {
    relative_from_mesh: [[f32; 4]; 4],
    joints: [[[f32; 4]; 4]; MAX_JOINTS],
}

struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    /// Index range and texture bind group per primitive.
    primitives: Vec<(std::ops::Range<u32>, wgpu::BindGroup)>,
}

pub struct Skinned {
    pipeline: wgpu::RenderPipeline,
    instance_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    meshes: HashMap<SkinnedMeshId, Mesh>,
}

impl Skinned {
    pub fn new(
        device: &wgpu::Device,
        view_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Skinned {
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skinned instance"),
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
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skinned texture"),
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
            ],
        });
        let pipeline = pipeline(
            device,
            format,
            PipelineSpec {
                label: "skinned",
                source: include_str!("shaders/skinned.wgsl"),
                layouts: &[view_layout, &instance_layout, &texture_layout],
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<SkinnedVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint16x4, 4 => Float32x4
                    ],
                })],
                solid: true,
            },
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("skinned"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Skinned {
            pipeline,
            instance_layout,
            texture_layout,
            sampler,
            meshes: HashMap::new(),
        }
    }

    pub fn apply(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, change: SkinnedChange) {
        match change {
            SkinnedChange::Remove(id) => {
                self.meshes.remove(&id);
            }
            SkinnedChange::Add(id, mesh) => {
                let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("skinned vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("skinned indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                let white = Image {
                    width: 1,
                    height: 1,
                    rgba: vec![255; 4],
                };
                let primitives = mesh
                    .primitives
                    .iter()
                    .map(|primitive| {
                        let image = primitive.image.map_or(&white, |index| &mesh.images[index]);
                        (
                            primitive.indices.clone(),
                            self.texture(device, queue, image),
                        )
                    })
                    .collect();
                self.meshes.insert(
                    id,
                    Mesh {
                        vertices,
                        indices,
                        primitives,
                    },
                );
            }
        }
    }

    fn texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &Image,
    ) -> wgpu::BindGroup {
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("skinned texture"),
                size: wgpu::Extent3d {
                    width: image.width,
                    height: image.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &image.rgba,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skinned texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Draws the instances [`InstanceUniforms::write`] placed, in the same order.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &InstanceUniforms,
        drawn: &[SkinnedMeshId],
    ) {
        if drawn.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        for (slot, id) in drawn.iter().enumerate() {
            let mesh = &self.meshes[id];
            pass.set_bind_group(1, &uniforms.slots[slot].1, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            for (indices, texture) in &mesh.primitives {
                pass.set_bind_group(2, texture, &[]);
                pass.draw_indexed(indices.clone(), 0, 0..1);
            }
        }
    }
}

/// One uniform block per skinned instance a view draws. Grows, never shrinks:
/// avatars in view are few.
#[derive(Default)]
pub struct InstanceUniforms {
    slots: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
}

impl InstanceUniforms {
    /// Writes the instances whose mesh the renderer holds and returns their
    /// meshes in slot order.
    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        skinned: &Skinned,
        instances: &[SkinnedInstance],
        camera: DVec3,
    ) -> Vec<SkinnedMeshId> {
        let mut drawn = Vec::new();
        for instance in instances
            .iter()
            .filter(|i| skinned.meshes.contains_key(&i.mesh))
        {
            let slot = drawn.len();
            if slot == self.slots.len() {
                self.slots
                    .push(Self::allocate(device, &skinned.instance_layout));
            }
            // Subtract the camera in f64, only then drop to f32.
            let relative = DAffine3::from_translation(-camera) * instance.transform;
            let mut uniform = InstanceUniform {
                relative_from_mesh: DMat4::from(relative).as_mat4().to_cols_array_2d(),
                joints: [glam::Mat4::IDENTITY.to_cols_array_2d(); MAX_JOINTS],
            };
            for (out, joint) in uniform.joints.iter_mut().zip(&instance.joints) {
                *out = joint.to_cols_array_2d();
            }
            queue.write_buffer(&self.slots[slot].0, 0, bytemuck::bytes_of(&uniform));
            drawn.push(instance.mesh);
        }
        drawn
    }

    fn allocate(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skinned instance"),
            size: size_of::<InstanceUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skinned instance"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        (buffer, bind_group)
    }
}
