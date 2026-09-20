//! Boxes: one unit cube, instanced.

use bytemuck::{Pod, Zeroable};
use glam::{DAffine3, DVec3};
use scene::BoxPart;
use wgpu::util::DeviceExt;

use crate::{PipelineSpec, Surface, pipeline};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct CubeVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Instance {
    relative_from_box: [[f32; 4]; 4],
    color: [f32; 4],
}

pub struct Boxes {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}

impl Boxes {
    pub fn new(
        device: &wgpu::Device,
        view_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Boxes {
        let buffers = &[
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<CubeVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4,
                    6 => Float32x4
                ],
            }),
        ];
        let pipeline = pipeline(
            device,
            format,
            PipelineSpec {
                label: "boxes",
                source: include_str!("shaders/boxes.wgsl"),
                layouts: &[view_layout],
                buffers,
                surface: Surface::Solid,
            },
        );
        let shadow_pipeline = crate::pipeline(
            device,
            format,
            PipelineSpec {
                label: "boxes",
                source: include_str!("shaders/boxes.wgsl"),
                layouts: &[shadow_layout],
                buffers,
                surface: Surface::Shadow,
            },
        );
        let (vertex_data, index_data) = cube();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube vertices"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube indices"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });
        Boxes {
            pipeline,
            shadow_pipeline,
            vertices,
            indices,
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, instances: &Instances, count: u32) {
        if count == 0 {
            return;
        }
        self.draw_with(pass, instances, count, &self.pipeline);
    }

    pub fn draw_shadow(&self, pass: &mut wgpu::RenderPass<'_>, instances: &Instances, count: u32) {
        if count == 0 {
            return;
        }
        self.draw_with(pass, instances, count, &self.shadow_pipeline);
    }

    fn draw_with(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        instances: &Instances,
        count: u32,
        pipeline: &wgpu::RenderPipeline,
    ) {
        pass.set_pipeline(pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..36, 0, 0..count);
    }
}

/// The boxes one view draws this frame, camera-relative.
pub struct Instances {
    buffer: wgpu::Buffer,
    capacity: usize,
}

impl Instances {
    pub fn new(device: &wgpu::Device) -> Instances {
        let capacity = 64;
        Instances {
            buffer: Self::allocate(device, capacity),
            capacity,
        }
    }

    fn allocate(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("box instances"),
            size: (size_of::<Instance>() * capacity) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        boxes: &[BoxPart],
        camera: DVec3,
    ) -> u32 {
        if boxes.len() > self.capacity {
            self.capacity = boxes.len().next_power_of_two();
            self.buffer = Self::allocate(device, self.capacity);
        }
        let data: Vec<Instance> = boxes
            .iter()
            .map(|part| {
                // Subtract the camera in f64, only then drop to f32.
                let relative = DAffine3::from_translation(-camera) * part.transform;
                Instance {
                    relative_from_box: glam::DMat4::from(relative).as_mat4().to_cols_array_2d(),
                    color: part.color.extend(1.0).to_array(),
                }
            })
            .collect();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
        boxes.len() as u32
    }
}

/// A unit cube centred on the origin: 24 vertices (flat normals), 36 indices,
/// counter clockwise from outside.
fn cube() -> (Vec<CubeVertex>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for axis in 0..3 {
        for sign in [1.0f32, -1.0] {
            let mut normal = [0.0; 3];
            normal[axis] = sign;
            // Two in-face axes ordered so that `a x b = normal`.
            let (a, b) = if sign > 0.0 {
                ((axis + 1) % 3, (axis + 2) % 3)
            } else {
                ((axis + 2) % 3, (axis + 1) % 3)
            };
            let base = vertices.len() as u16;
            for (sa, sb) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
                let mut position = [0.0; 3];
                position[axis] = sign * 0.5;
                position[a] = sa;
                position[b] = sb;
                vertices.push(CubeVertex { position, normal });
            }
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    (vertices, indices)
}
