//! Portal panes: the disc of a ring, a window on the world seen through it.
//! The world past a portal is drawn first, from the eye carried through it,
//! into a target of its own at a quarter of the view's height; a pane the
//! view is seen through then samples that picture where the pane stands on
//! the screen. The eye past the portal shares the view's projection, so a
//! point of the pane and the point seen through it fall on the same pixel,
//! and the pane is a window, with parallax (Valve's Portal draws its
//! portals so). A pane with nothing seen through it is a lens: what is
//! behind it, bent toward its middle and pulsing, as through thick glass,
//! so a portal is a portal from afar too. Drawn after the sea and the
//! clouds over a copy of the picture so far, which the lens reads. A pane
//! casts no shadow: light passes a portal.

use bytemuck::{Pod, Zeroable};
use glam::{DAffine3, DVec3};
use scene::PaneDraw;
use wgpu::util::DeviceExt;

use crate::pictures::{BOTH, QuadVertex, quad};
use crate::{PipelineSpec, Surface, pipeline};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Instance {
    relative_from_quad: [[f32; 4]; 4],
    /// `x`: how much the world seen through the portal shows in this pane,
    /// 0 a lens to 1 a window.
    params: [f32; 4],
}

pub struct Panes {
    pipeline: wgpu::RenderPipeline,
    through_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    /// What a pane samples while nothing is seen through any: one texel.
    none: wgpu::BindGroup,
}

impl Panes {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_layout: &wgpu::BindGroupLayout,
        behind_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Panes {
        let through_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("through"),
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
        let buffers = &[
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<QuadVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
                    7 => Float32x4
                ],
            }),
        ];
        let pipeline = pipeline(
            device,
            format,
            PipelineSpec {
                label: "panes",
                source: include_str!("shaders/pane.wgsl"),
                layouts: &[view_layout, &through_layout, behind_layout],
                buffers,
                surface: Surface::Lens,
            },
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("through"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let (vertex_data, index_data) = quad();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pane quad"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pane quad indices"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });
        let none = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("nothing through"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::compose::SCENE_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[0; 8],
        );
        let none = none.create_view(&wgpu::TextureViewDescriptor::default());
        let none = through_group(device, &through_layout, &sampler, &none);
        Panes {
            pipeline,
            through_layout,
            sampler,
            vertices,
            indices,
            none,
        }
    }

    /// What the panes sample: the picture of the world past the portal.
    pub fn through_group(
        &self,
        device: &wgpu::Device,
        picture: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        through_group(device, &self.through_layout, &self.sampler, picture)
    }

    /// Draws every pane [`Instances::write`] placed, in order, sampling the
    /// world seen through the portal when one is drawn this frame, in a
    /// pass with the copy of the picture behind them bound as group 2.
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        instances: &Instances,
        drawn: &[PaneDraw],
        through: Option<&wgpu::BindGroup>,
    ) {
        if drawn.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, through.unwrap_or(&self.none), &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..BOTH, 0, 0..drawn.len() as u32);
    }
}

fn through_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    picture: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("through"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(picture),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

/// The panes one view draws this frame, camera-relative.
pub struct Instances {
    buffer: wgpu::Buffer,
    capacity: usize,
}

impl Instances {
    pub fn new(device: &wgpu::Device) -> Instances {
        let capacity = 16;
        Instances {
            buffer: Self::allocate(device, capacity),
            capacity,
        }
    }

    fn allocate(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pane instances"),
            size: (size_of::<Instance>() * capacity) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        panes: &[PaneDraw],
        camera: DVec3,
    ) {
        if panes.is_empty() {
            return;
        }
        if panes.len() > self.capacity {
            self.capacity = panes.len().next_power_of_two();
            self.buffer = Self::allocate(device, self.capacity);
        }
        let data: Vec<Instance> = panes
            .iter()
            .map(|pane| {
                // Subtract the camera in f64, only then drop to f32.
                let relative = DAffine3::from_translation(-camera) * pane.transform;
                Instance {
                    relative_from_quad: glam::DMat4::from(relative).as_mat4().to_cols_array_2d(),
                    params: [pane.seen, 0.0, 0.0, 0.0],
                }
            })
            .collect();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
    }
}
