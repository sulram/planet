//! Volume meshes: the cubes of the build layer, solid and of glass, the
//! ghost of a stroke, and the guides that show where cells are. One vertex
//! and index buffer each, placed by the same per view ring of offsets
//! terrain uses. And the lamps among the cells, which light every surface.

use std::collections::HashMap;

use glam::DVec3;
use scene::{Frame, GuideVertex, Lamp, VolumeChange, VolumeDraw, VolumeMeshId, VolumeVertex};
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

/// The most lamps one view is lit by: the nearest.
pub const LAMPS: usize = 16;

/// What the shaders must agree on, stated once: prepended to every module.
pub fn prelude() -> String {
    format!("const LAMPS: u32 = {LAMPS}u;\n")
}

/// The lamps that light a view, as its uniform holds them: the nearest to
/// the camera, each where it is from there, and how many they are.
pub fn lamps(lamps: &[Lamp], camera: DVec3) -> ([[[f32; 4]; 2]; LAMPS], f32) {
    // How far the camera is past where a lamp's light ends.
    let beyond = |lamp: &Lamp| lamp.position.distance(camera) - f64::from(lamp.reach_m);
    let mut near: Vec<&Lamp> = lamps.iter().collect();
    near.sort_by(|a, b| beyond(a).total_cmp(&beyond(b)));
    let mut lit = [[[0.0; 4]; 2]; LAMPS];
    for (slot, lamp) in lit.iter_mut().zip(&near) {
        let at = (lamp.position - camera).as_vec3();
        *slot = [
            at.extend(lamp.reach_m).to_array(),
            lamp.color.extend(0.0).to_array(),
        ];
    }
    (lit, near.len().min(LAMPS) as f32)
}

pub struct Volumes {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
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
                0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Unorm8x4
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
        let guide = |surface: Surface| {
            pipeline(
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
                            0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4, 3 => Unorm8x4,
                            4 => Unorm8x4
                        ],
                    })],
                    surface,
                },
            )
        };
        Volumes {
            pipeline: make(view_layout, Surface::Solid),
            shadow_pipeline: make(shadow_layout, Surface::Shadow),
            glass_pipeline: make(view_layout, Surface::Glass),
            ghost_pipeline: guide(Surface::Ghost),
            guide_pipeline: guide(Surface::Guide),
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
    /// the glass from the farthest to the nearest, then the guides, then the
    /// ghost, and returns them in slot order. A mesh named before it was
    /// applied is skipped, never a panic.
    pub fn place(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniforms: &mut PatchUniforms,
        frame: &Frame,
        camera: DVec3,
    ) -> Placed {
        let held = |wanted: &[VolumeDraw]| -> Vec<VolumeDraw> {
            let held = wanted
                .iter()
                .filter(|draw| self.meshes.contains_key(&draw.id));
            held.copied().collect()
        };
        let solid = held(&frame.volumes);
        // Glass blends over glass: the far panes go first.
        let mut glass = held(&frame.glass);
        let at = |draw: &VolumeDraw| draw.body_center + self.meshes[&draw.id].origin;
        let away = |draw: &VolumeDraw| at(draw).distance_squared(camera);
        glass.sort_by(|a, b| away(b).total_cmp(&away(a)));
        let guides = held(&frame.guides);
        let ghost = frame
            .ghost
            .filter(|draw| self.meshes.contains_key(&draw.id));
        // A mesh is counted from the centre of its body, which moves.
        let drawn = solid.iter().chain(&glass).chain(&guides).chain(&ghost);
        let origins: Vec<(DVec3, DVec3)> = drawn
            .map(|draw| (draw.body_center, self.meshes[&draw.id].origin))
            .collect();
        uniforms.place(device, queue, &origins, camera);
        let ids = |draws: Vec<VolumeDraw>| draws.into_iter().map(|draw| draw.id).collect();
        Placed {
            solid: ids(solid),
            glass: ids(glass),
            guides: ids(guides),
            ghost: ghost.map(|draw| draw.id),
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

    /// The glass, the guides and the ghost over them, after everything
    /// opaque, the sea and the clouds: they test depth and write none, so
    /// what stands in front of them hides them and they hide nothing.
    pub fn draw_over(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniforms: &PatchUniforms,
        placed: &Placed,
    ) {
        let mut slot = placed.solid.len();
        let over = [
            (&self.glass_pipeline, placed.glass.as_slice()),
            (&self.guide_pipeline, placed.guides.as_slice()),
            (&self.ghost_pipeline, placed.ghost.as_slice()),
        ];
        for (pipeline, meshes) in over {
            if !meshes.is_empty() {
                pass.set_pipeline(pipeline);
            }
            for id in meshes {
                self.draw_one(pass, uniforms, slot, id);
                slot += 1;
            }
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
    glass: Vec<VolumeMeshId>,
    guides: Vec<VolumeMeshId>,
    ghost: Option<VolumeMeshId>,
}

impl Placed {
    /// Whether anything is drawn over the picture once the sea and the
    /// clouds are in it.
    pub fn over(&self) -> bool {
        !self.glass.is_empty() || !self.guides.is_empty() || self.ghost.is_some()
    }
}
