//! Pictures: a textured quad each, one draw a picture with its own texture.
//! An image arrives as its one full level and gets its mip chain here, on the
//! GPU, so a picture seen small or aslant does not shimmer. A picture that
//! moves is a texture the shell writes a frame at a time, its mip chain
//! redrawn with every frame.

use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{DAffine3, DVec3};
use scene::{Image, PictureChange, PictureDraw, PictureId};
use wgpu::util::DeviceExt;

use crate::{PipelineSpec, Surface, pipeline};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct QuadVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Instance {
    relative_from_quad: [[f32; 4]; 4],
    /// `x`: how much the picture shines with its own colour.
    params: [f32; 4],
}

/// Indices of the front of the quad, then of its back.
const FRONT: u32 = 6;
pub(crate) const BOTH: u32 = 12;

/// The format every picture is kept in: sRGB bytes, read as linear light.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// The light grey of the card a picture is before its first byte lands.
const CARD: [u8; 4] = [200, 200, 200, 255];

/// One frame of a picture that moves, as the shell hands it: pixels, or on
/// the web an image the browser holds, copied into the texture without
/// passing through the engine.
pub enum PictureFrame<'a> {
    Pixels(&'a Image),
    #[cfg(target_arch = "wasm32")]
    External(wgpu::CopyExternalImageSourceInfo),
}

impl PictureFrame<'_> {
    fn size(&self) -> (u32, u32) {
        match self {
            PictureFrame::Pixels(image) => (image.width, image.height),
            #[cfg(target_arch = "wasm32")]
            PictureFrame::External(info) => (info.source.width(), info.source.height()),
        }
    }
}

/// A picture resident on the GPU: its texture, kept for the frames a moving
/// one takes, and the bind group that draws it.
struct Held {
    texture: wgpu::Texture,
    group: wgpu::BindGroup,
}

pub struct Pictures {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    /// The card a picture is before its first byte lands.
    blank: wgpu::BindGroup,
    held: HashMap<PictureId, Held>,
    mips: Mips,
    /// The widest texture the device takes on a side.
    max_side: u32,
}

impl Pictures {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Pictures {
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("picture"),
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
                label: "pictures",
                source: include_str!("shaders/picture.wgsl"),
                layouts: &[view_layout, &texture_layout],
                buffers,
                surface: Surface::Solid,
            },
        );
        let shadow_pipeline = crate::pipeline(
            device,
            format,
            PipelineSpec {
                label: "pictures",
                source: include_str!("shaders/picture.wgsl"),
                layouts: &[shadow_layout, &texture_layout],
                buffers,
                surface: Surface::ShadowCutout,
            },
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("picture"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });
        let (vertex_data, index_data) = quad();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("picture quad"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("picture quad indices"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });
        let mips = Mips::new(device);
        let blank = mips.bind_group(
            device,
            &texture_layout,
            &sampler,
            &blank_texture(device, queue),
        );
        Pictures {
            pipeline,
            shadow_pipeline,
            texture_layout,
            sampler,
            vertices,
            indices,
            blank,
            held: HashMap::new(),
            mips,
            max_side: device.limits().max_texture_dimension_2d,
        }
    }

    /// Uploads a picture with its mip chain, makes the texture of one that
    /// moves, or drops one.
    pub fn apply(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, change: PictureChange) {
        match change {
            PictureChange::Remove(id) => {
                self.held.remove(&id);
            }
            PictureChange::Add(id, image) => {
                let image = fit(&image, self.max_side);
                let texture = self.mips.create(device, image.width, image.height);
                self.mips.write(queue, &texture, &image);
                self.mips.generate(device, queue, &texture);
                self.hold(device, id, texture);
            }
            PictureChange::Live { id, width, height } => {
                let (width, height) = (width.max(1), height.max(1));
                if width.max(height) > self.max_side {
                    log::warn!("picture {id:?}: {width}x{height} is wider than the device takes");
                    return;
                }
                let texture = self.mips.create(device, width, height);
                let card = Image {
                    width,
                    height,
                    rgba: CARD.repeat((width * height) as usize),
                };
                self.mips.write(queue, &texture, &card);
                self.mips.generate(device, queue, &texture);
                self.hold(device, id, texture);
            }
        }
    }

    fn hold(&mut self, device: &wgpu::Device, id: PictureId, texture: wgpu::Texture) {
        let group = self
            .mips
            .bind_group(device, &self.texture_layout, &self.sampler, &texture);
        self.held.insert(id, Held { texture, group });
    }

    /// One frame of a picture that moves, written over the last and its mip
    /// chain redrawn. A frame of another size than the texture is dropped: a
    /// source says its size when it opens and keeps it.
    pub fn write(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: PictureId,
        frame: PictureFrame<'_>,
    ) {
        let Some(held) = self.held.get(&id) else {
            return;
        };
        let size = (held.texture.width(), held.texture.height());
        if frame.size() != size {
            log::warn!(
                "picture {id:?}: a frame of {:?} for a texture of {size:?}",
                frame.size()
            );
            return;
        }
        match frame {
            PictureFrame::Pixels(image) => self.mips.write(queue, &held.texture, image),
            #[cfg(target_arch = "wasm32")]
            PictureFrame::External(source) => queue.copy_external_image_to_texture(
                &source,
                wgpu::CopyExternalImageDestInfo {
                    texture: &held.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                    color_space: wgpu::PredefinedColorSpace::Srgb,
                    premultiplied_alpha: false,
                },
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
            ),
        }
        self.mips.generate(device, queue, &held.texture);
    }

    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        instances: &Instances,
        drawn: &[PictureDraw],
    ) {
        self.draw_with(pass, instances, drawn, &self.pipeline);
    }

    pub fn draw_shadow(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        instances: &Instances,
        drawn: &[PictureDraw],
    ) {
        self.draw_with(pass, instances, drawn, &self.shadow_pipeline);
    }

    fn draw_with(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        instances: &Instances,
        drawn: &[PictureDraw],
        pipeline: &wgpu::RenderPipeline,
    ) {
        if drawn.is_empty() {
            return;
        }
        pass.set_pipeline(pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        for (index, picture) in drawn.iter().enumerate() {
            let group = picture
                .picture
                .and_then(|id| self.held.get(&id))
                .map_or(&self.blank, |held| &held.group);
            pass.set_bind_group(1, group, &[]);
            let sides = if picture.both_sides { BOTH } else { FRONT };
            let index = index as u32;
            pass.draw_indexed(0..sides, 0, index..index + 1);
        }
    }
}

/// The pictures one view draws this frame, camera-relative.
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
            label: Some("picture instances"),
            size: (size_of::<Instance>() * capacity) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pictures: &[PictureDraw],
        camera: DVec3,
    ) {
        if pictures.is_empty() {
            return;
        }
        if pictures.len() > self.capacity {
            self.capacity = pictures.len().next_power_of_two();
            self.buffer = Self::allocate(device, self.capacity);
        }
        let data: Vec<Instance> = pictures
            .iter()
            .map(|picture| {
                // Subtract the camera in f64, only then drop to f32.
                let relative = DAffine3::from_translation(-camera) * picture.transform;
                Instance {
                    relative_from_quad: glam::DMat4::from(relative).as_mat4().to_cols_array_2d(),
                    params: [picture.shine, 0.0, 0.0, 0.0],
                }
            })
            .collect();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
    }
}

/// The mip chain maker: a pass a level, each reading the finer one.
struct Mips {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl Mips {
    fn new(device: &wgpu::Device) -> Mips {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mip"),
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
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mip"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mip.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mip"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mip"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Mips {
            pipeline,
            layout,
            sampler,
        }
    }

    /// A texture of this size with every level down to one texel, nothing
    /// written yet.
    fn create(&self, device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        let levels = 32 - width.max(height).max(1).leading_zeros();
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("picture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    /// The image into the texture's first level, which is its size.
    fn write(&self, queue: &wgpu::Queue, texture: &wgpu::Texture, image: &Image) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width),
                rows_per_image: Some(image.height),
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Every level under the first drawn from the one above it.
    fn generate(&self, device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
        let levels = texture.mip_level_count();
        let level = |level: u32| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mips"),
        });
        for coarser in 1..levels {
            let finer = level(coarser - 1);
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mip"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&finer),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let target = level(coarser);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mip"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        texture: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("picture"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

/// One light grey texel: the card of a picture not yet landed.
fn blank_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("blank picture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &CARD,
    )
}

/// The image, halved until the device takes it. A client asks for pictures
/// no wider than it needs; this holds whatever arrives.
fn fit(image: &Arc<Image>, max_side: u32) -> Arc<Image> {
    let mut image = Arc::clone(image);
    while image.width.max(image.height) > max_side && image.width > 1 && image.height > 1 {
        image = Arc::new(halve(&image));
    }
    image
}

/// Every two by two texels averaged into one, in sRGB bytes: a step down
/// taken on the CPU, so a texture wider than the device takes still lands.
fn halve(image: &Image) -> Image {
    let (width, height) = (image.width / 2, image.height / 2);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let at = |x: u32, y: u32| {
        let i = ((y * image.width + x) * 4) as usize;
        &image.rgba[i..i + 4]
    };
    for y in 0..height {
        for x in 0..width {
            let quad = [
                at(2 * x, 2 * y),
                at(2 * x + 1, 2 * y),
                at(2 * x, 2 * y + 1),
                at(2 * x + 1, 2 * y + 1),
            ];
            for channel in 0..4 {
                let sum: u32 = quad.iter().map(|texel| u32::from(texel[channel])).sum();
                rgba.push(((sum + 2) / 4) as u8);
            }
        }
    }
    Image {
        width,
        height,
        rgba,
    }
}

/// A unit quad in the `xy` plane: its front facing `+z`, then its back facing
/// `-z` with the image reading the right way round from behind.
pub(crate) fn quad() -> (Vec<QuadVertex>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(8);
    let mut indices = Vec::with_capacity(BOTH as usize);
    for sign in [1.0f32, -1.0] {
        let base = vertices.len() as u16;
        for (x, y) in [(-0.5f32, -0.5f32), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            vertices.push(QuadVertex {
                position: [x, y, 0.0],
                normal: [0.0, 0.0, sign],
                // The top row of the texture is the top of the quad. Seen
                // from behind, left is still left.
                uv: [0.5 + sign * x, 0.5 - y],
            });
        }
        if sign > 0.0 {
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        } else {
            indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
    (vertices, indices)
}
