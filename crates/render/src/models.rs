//! Model meshes: the cubes of parts, each mesh in its own space and placed
//! by one matrix a frame, relative to the camera, as an avatar is. The
//! sides are the volumes' own (`shaders/cubes.wgsl`), so a part is lit,
//! edged and glazed as the cells are.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use glam::{DAffine3, DMat4, DVec3, Vec3};
use scene::{ModelChange, ModelDraw, ModelId, ModelPass, VolumeVertex};
use wgpu::util::DeviceExt;

use crate::{PipelineSpec, Surface, pipeline};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceUniform {
    relative_from_model: [[f32; 4]; 4],
    normal_from_model: [[f32; 4]; 4],
}

struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    /// How far a corner reaches from the mesh's anchor, in its own units.
    reach: f32,
}

pub struct Models {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
    over_pipeline: wgpu::RenderPipeline,
    instance_layout: wgpu::BindGroupLayout,
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
        view_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
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
        Models {
            pipeline: make(view_layout, Surface::Solid),
            shadow_pipeline: make(shadow_layout, Surface::Shadow),
            glass_pipeline: make(view_layout, Surface::Glass),
            over_pipeline: make(view_layout, Surface::Overlay),
            instance_layout,
            meshes: HashMap::new(),
        }
    }

    pub fn apply(&mut self, device: &wgpu::Device, change: ModelChange) {
        match change {
            ModelChange::Add(id, mesh) if !mesh.indices.is_empty() => {
                let reach = mesh
                    .vertices
                    .iter()
                    .map(|v| Vec3::from(v.position).length())
                    .fold(0.0, f32::max);
                let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                self.meshes.insert(
                    id,
                    Mesh {
                        vertices,
                        indices,
                        count: mesh.indices.len() as u32,
                        reach,
                    },
                );
            }
            ModelChange::Add(id, _) | ModelChange::Remove(id) => {
                self.meshes.remove(&id);
            }
        }
    }

    /// Draws the solid models [`InstanceUniforms::write`] placed.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &InstanceUniforms,
        placed: &Placed,
    ) {
        if placed.solid.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        for &(slot, id, _, _) in &placed.solid {
            self.draw_one(pass, uniforms, slot, id);
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
            .collect();
        if casting.is_empty() {
            return;
        }
        pass.set_pipeline(&self.shadow_pipeline);
        for &(slot, id, _, _) in casting {
            self.draw_one(pass, uniforms, slot, id);
        }
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
            for &(slot, id) in drawn {
                self.draw_one(pass, uniforms, slot, id);
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
        pass.draw_indexed(0..mesh.count, 0, 0..1);
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
