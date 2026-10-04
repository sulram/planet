//! Volume meshes: the cubes of the build layer, the ghost of a stroke, and
//! the guides that show where cells are. One vertex and index buffer each,
//! placed by the same per view ring of offsets terrain uses.

use std::collections::HashMap;

use glam::DVec3;
use scene::{Frame, GuideVertex, VolumeChange, VolumeMeshId, VolumeVertex};
use wgpu::util::DeviceExt;

use crate::terrain::PatchUniforms;
use crate::{PipelineSpec, Surface, pipeline};

struct Mesh {
    origin: DVec3,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    radius_m: f32,
}

pub struct Volumes {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    ghost_pipeline: wgpu::RenderPipeline,
    guide_pipeline: wgpu::RenderPipeline,
    /// Every mesh held, cubes and guides alike: a frame says which is drawn
    /// as what.
    meshes: HashMap<VolumeMeshId, Mesh>,
}

impl Volumes {
    pub fn new(
        device: &wgpu::Device,
        view_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
        placement_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Volumes {
        let buffers = &[Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<VolumeVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Float32
            ],
        })];
        let make = |layout: &wgpu::BindGroupLayout, surface: Surface| {
            pipeline(
                device,
                format,
                PipelineSpec {
                    label: "volume",
                    source: include_str!("shaders/volume.wgsl"),
                    layouts: &[layout, placement_layout],
                    buffers,
                    surface,
                },
            )
        };
        let guide_pipeline = pipeline(
            device,
            format,
            PipelineSpec {
                label: "guide",
                source: include_str!("shaders/guide.wgsl"),
                layouts: &[view_layout, placement_layout],
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<GuideVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4
                    ],
                })],
                surface: Surface::Guide,
            },
        );
        Volumes {
            pipeline: make(view_layout, Surface::Solid),
            shadow_pipeline: make(shadow_layout, Surface::Shadow),
            ghost_pipeline: make(view_layout, Surface::Ghost),
            guide_pipeline,
            meshes: HashMap::new(),
        }
    }

    fn hold(
        &mut self,
        device: &wgpu::Device,
        id: VolumeMeshId,
        origin: DVec3,
        vertices: &[u8],
        indices: &[u32],
        radius_m: f32,
    ) {
        if indices.is_empty() {
            self.meshes.remove(&id);
            return;
        }
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("volume"),
            contents: vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let count = indices.len() as u32;
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("volume indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.meshes.insert(
            id,
            Mesh {
                origin,
                vertices,
                indices,
                count,
                radius_m,
            },
        );
    }

    pub fn apply(&mut self, device: &wgpu::Device, change: VolumeChange) {
        let reach = |positions: &mut dyn Iterator<Item = [f32; 3]>| {
            positions
                .map(|at| glam::Vec3::from(at).length())
                .fold(0.0, f32::max)
        };
        match change {
            VolumeChange::Add(id, mesh) => {
                let radius_m = reach(&mut mesh.vertices.iter().map(|v| v.position));
                let vertices = bytemuck::cast_slice(&mesh.vertices);
                self.hold(device, id, mesh.origin, vertices, &mesh.indices, radius_m);
            }
            VolumeChange::Guide(id, mesh) => {
                let radius_m = reach(&mut mesh.vertices.iter().map(|v| v.position));
                let vertices = bytemuck::cast_slice(&mesh.vertices);
                self.hold(device, id, mesh.origin, vertices, &mesh.indices, radius_m);
            }
            VolumeChange::Remove(id) => {
                self.meshes.remove(&id);
            }
        }
    }

    /// Places every mesh a frame names that this renderer holds, the cubes,
    /// then the guides, then the ghost, and returns them in slot order. A
    /// mesh named before it was applied is skipped, never a panic.
    pub fn place(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniforms: &mut PatchUniforms,
        frame: &Frame,
        camera: DVec3,
    ) -> Placed {
        let held = |wanted: &[VolumeMeshId]| -> Vec<VolumeMeshId> {
            let held = wanted.iter().filter(|id| self.meshes.contains_key(id));
            held.copied().collect()
        };
        let solid = held(&frame.volumes);
        let guides = held(&frame.guides);
        let ghost = frame.ghost.filter(|id| self.meshes.contains_key(id));
        // Volumes stand on the planet, whose centre is the world origin.
        let origins: Vec<(DVec3, DVec3)> = solid
            .iter()
            .chain(&guides)
            .chain(&ghost)
            .map(|id| (DVec3::ZERO, self.meshes[id].origin))
            .collect();
        uniforms.place(device, queue, &origins, camera);
        Placed {
            solid,
            guides,
            ghost,
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, uniforms: &PatchUniforms, placed: &Placed) {
        pass.set_pipeline(&self.pipeline);
        for (slot, id) in placed.solid.iter().enumerate() {
            self.draw_one(pass, uniforms, slot, id);
        }
    }

    pub fn draw_shadow(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &PatchUniforms,
        placed: &Placed,
        matrix: glam::Mat4,
    ) {
        pass.set_pipeline(&self.shadow_pipeline);
        for (slot, id) in placed.solid.iter().enumerate() {
            let center = uniforms.centers()[slot];
            if crate::shadow::intersects(matrix, center, self.meshes[id].radius_m) {
                self.draw_one(pass, uniforms, slot, id);
            }
        }
    }

    /// The guides and the ghost over them, after everything opaque and the
    /// sky: they test depth and write none, so what stands in front of them
    /// hides them and they hide nothing.
    pub fn draw_over(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &PatchUniforms,
        placed: &Placed,
    ) {
        let first = placed.solid.len();
        if !placed.guides.is_empty() {
            pass.set_pipeline(&self.guide_pipeline);
        }
        for (slot, id) in placed.guides.iter().enumerate() {
            self.draw_one(pass, uniforms, first + slot, id);
        }
        if let Some(id) = &placed.ghost {
            pass.set_pipeline(&self.ghost_pipeline);
            self.draw_one(pass, uniforms, first + placed.guides.len(), id);
        }
    }

    fn draw_one(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &PatchUniforms,
        slot: usize,
        id: &VolumeMeshId,
    ) {
        let mesh = &self.meshes[id];
        pass.set_bind_group(1, uniforms.bind_group(), &[slot as u32 * uniforms.stride()]);
        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.count, 0, 0..1);
    }
}

/// What one view draws of the volumes this frame, in slot order.
pub struct Placed {
    solid: Vec<VolumeMeshId>,
    guides: Vec<VolumeMeshId>,
    ghost: Option<VolumeMeshId>,
}
