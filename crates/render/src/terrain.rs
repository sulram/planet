//! Terrain patches: one vertex buffer each, one shared index buffer, and a
//! per view ring of offsets bound with a dynamic offset.

use std::collections::HashMap;

use glam::DVec3;
use scene::{PatchDraw, PatchId, TerrainChange, TerrainVertex, WaterVertex, patch_indices};
use wgpu::util::DeviceExt;

use crate::{PipelineSpec, Surface, pipeline, relative};

struct Patch {
    origin: DVec3,
    vertices: wgpu::Buffer,
    water: Option<wgpu::Buffer>,
}

pub struct Terrain {
    pipeline: wgpu::RenderPipeline,
    water_pipeline: wgpu::RenderPipeline,
    patch_layout: wgpu::BindGroupLayout,
    indices: wgpu::Buffer,
    index_count: u32,
    patches: HashMap<PatchId, Patch>,
}

impl Terrain {
    pub fn new(
        device: &wgpu::Device,
        view_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Terrain {
        let patch_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("patch"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(SLOT_BYTES),
                },
                count: None,
            }],
        });
        let ground_pipeline = pipeline(
            device,
            format,
            PipelineSpec {
                label: "terrain",
                source: include_str!("shaders/terrain.wgsl"),
                layouts: &[view_layout, &patch_layout],
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<TerrainVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4
                    ],
                })],
                surface: Surface::Solid,
            },
        );
        let water_pipeline = pipeline(
            device,
            format,
            PipelineSpec {
                label: "water",
                source: include_str!("shaders/water.wgsl"),
                layouts: &[view_layout, &patch_layout],
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<WaterVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32],
                })],
                surface: Surface::Translucent,
            },
        );
        let index_data = patch_indices();
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("patch indices"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });
        Terrain {
            pipeline: ground_pipeline,
            water_pipeline,
            patch_layout,
            indices,
            index_count: index_data.len() as u32,
            patches: HashMap::new(),
        }
    }

    pub fn patch_layout(&self) -> &wgpu::BindGroupLayout {
        &self.patch_layout
    }

    pub fn apply(&mut self, device: &wgpu::Device, change: TerrainChange) {
        match change {
            TerrainChange::Add(id, mesh) => {
                let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("patch"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let water = mesh.water.map(|water| {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("patch water"),
                        contents: bytemuck::cast_slice(&water),
                        usage: wgpu::BufferUsages::VERTEX,
                    })
                });
                self.patches.insert(
                    id,
                    Patch {
                        origin: mesh.origin,
                        vertices,
                        water,
                    },
                );
            }
            TerrainChange::Remove(id) => {
                self.patches.remove(&id);
            }
        }
    }

    /// Draws the patches [`PatchUniforms::write`] placed, in the same order.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &PatchUniforms,
        drawn: &[PatchId],
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        for (slot, id) in drawn.iter().enumerate() {
            let patch = &self.patches[id];
            pass.set_bind_group(1, &uniforms.bind_group, &[slot as u32 * uniforms.stride]);
            pass.set_vertex_buffer(0, patch.vertices.slice(..));
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }

    /// Draws the sea over the same patches, after everything opaque.
    pub fn draw_water(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &PatchUniforms,
        drawn: &[PatchId],
    ) {
        pass.set_pipeline(&self.water_pipeline);
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        for (slot, id) in drawn.iter().enumerate() {
            let Some(water) = &self.patches[id].water else {
                continue;
            };
            pass.set_bind_group(1, &uniforms.bind_group, &[slot as u32 * uniforms.stride]);
            pass.set_vertex_buffer(0, water.slice(..));
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }
}

/// Bytes the shader reads per patch: two `vec4<f32>`, offset and anchor.
const SLOT_BYTES: u64 = 32;

/// Detail noise repeats every this many metres, so an origin wrapped to it in
/// f64 anchors the detail to the planet. Must match `ANCHOR_M` in the shaders.
const ANCHOR_M: f64 = 1024.0;

/// The camera-relative offsets of the patches one view draws this frame.
pub struct PatchUniforms {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    layout: wgpu::BindGroupLayout,
    /// Bytes between slots: the device's dynamic offset alignment.
    stride: u32,
    capacity: usize,
}

impl PatchUniforms {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> PatchUniforms {
        let stride = device
            .limits()
            .min_uniform_buffer_offset_alignment
            .max(SLOT_BYTES as u32);
        let capacity = 512;
        let (buffer, bind_group) = Self::allocate(device, layout, stride, capacity);
        PatchUniforms {
            buffer,
            bind_group,
            layout: layout.clone(),
            stride,
            capacity,
        }
    }

    fn allocate(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        stride: u32,
        capacity: usize,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("patch offsets"),
            size: u64::from(stride) * capacity as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("patch offsets"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(SLOT_BYTES),
                }),
            }],
        });
        (buffer, bind_group)
    }

    /// Writes one offset per patch the renderer holds, and returns those
    /// patches in slot order. A patch the client names before its mesh was
    /// applied is skipped, never a panic.
    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        terrain: &Terrain,
        wanted: &[PatchDraw],
        camera: DVec3,
    ) -> Vec<PatchId> {
        let placed: Vec<&PatchDraw> = wanted
            .iter()
            .filter(|draw| terrain.patches.contains_key(&draw.id))
            .collect();
        let drawn: Vec<PatchId> = placed.iter().map(|draw| draw.id).collect();
        if drawn.len() > self.capacity {
            self.capacity = drawn.len().next_power_of_two();
            (self.buffer, self.bind_group) =
                Self::allocate(device, &self.layout, self.stride, self.capacity);
        }
        let mut bytes = vec![0u8; drawn.len() * self.stride as usize];
        for (slot, draw) in placed.iter().enumerate() {
            // A patch is built around its body's centre; the body is wherever
            // it is this frame.
            let origin = terrain.patches[&draw.id].origin;
            let offset = relative(draw.body_center + origin, camera).extend(0.0);
            // Wrapped in f64: exact, however far from the body's centre. It is
            // the body relative origin that is wrapped, so detail is fixed to
            // the ground of a moving moon too.
            let anchor = origin
                .rem_euclid(DVec3::splat(ANCHOR_M))
                .as_vec3()
                .extend(0.0);
            let at = slot * self.stride as usize;
            bytes[at..at + SLOT_BYTES as usize].copy_from_slice(bytemuck::cast_slice(&[
                offset.to_array(),
                anchor.to_array(),
            ]));
        }
        queue.write_buffer(&self.buffer, 0, &bytes);
        drawn
    }
}
