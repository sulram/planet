//! Our egui backend on wgpu: egui's paint contract is textured triangles and
//! scissor rectangles, so this is one pipeline, two growing buffers and a map
//! of textures. `egui-wgpu` would do, the day it rides our wgpu.

use std::collections::HashMap;

/// A buffer that is replaced by a larger one when a frame needs more room.
struct Growing {
    buffer: wgpu::Buffer,
    capacity: u64,
    usage: wgpu::BufferUsages,
}

impl Growing {
    fn new(device: &wgpu::Device, usage: wgpu::BufferUsages, capacity: u64) -> Growing {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("panel geometry"),
            size: capacity,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Growing {
            buffer,
            capacity,
            usage,
        }
    }

    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) {
        let needed = bytes.len() as u64;
        if needed > self.capacity {
            *self = Growing::new(device, self.usage, needed.next_power_of_two());
        }
        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Screen {
    size_points: [f32; 2],
    linear: f32,
    pad: f32,
}

/// One draw: a run of indices, under one texture, inside one rectangle.
struct Draw {
    texture: egui::TextureId,
    indices: core::ops::Range<u32>,
    base_vertex: i32,
    clip: egui::Rect,
}

pub struct Painter {
    pipeline: wgpu::RenderPipeline,
    screen: wgpu::Buffer,
    screen_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<egui::TextureId, (wgpu::Texture, wgpu::BindGroup)>,
    vertices: Growing,
    indices: Growing,
    /// The target is sRGB: colours are made linear on the way out.
    linear: bool,
}

impl Painter {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Painter {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("panel"),
            source: wgpu::ShaderSource::Wgsl(include_str!("panel.wgsl").into()),
        });
        let screen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("panel screen"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("panel texture"),
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("panel"),
            bind_group_layouts: &[Some(&screen_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("panel"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                // egui's vertex: position, uv, colour as four bytes.
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<egui::epaint::Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // egui's colours are premultiplied.
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let screen = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("panel screen"),
            size: size_of::<Screen>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let screen_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("panel screen"),
            layout: &screen_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen.as_entire_binding(),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("panel"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Painter {
            pipeline,
            screen,
            screen_group,
            texture_layout,
            sampler,
            textures: HashMap::new(),
            vertices: Growing::new(device, wgpu::BufferUsages::VERTEX, 1 << 16),
            indices: Growing::new(device, wgpu::BufferUsages::INDEX, 1 << 16),
            linear: format.is_srgb(),
        }
    }

    /// Paints over `target`, keeping what is there.
    #[allow(clippy::too_many_arguments)]
    pub fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
        pixels_per_point: f32,
        primitives: &[egui::ClippedPrimitive],
        textures: &egui::TexturesDelta,
    ) {
        for (id, delta) in &textures.set {
            self.set_texture(device, queue, *id, delta);
        }

        // Every mesh into one vertex buffer and one index buffer.
        let mut vertex_bytes: Vec<u8> = Vec::new();
        let mut index_bytes: Vec<u8> = Vec::new();
        let mut draws = Vec::new();
        let (mut vertex_count, mut index_count) = (0i32, 0u32);
        for clipped in primitives {
            // Callbacks are an integration's own drawing; this panel has none.
            let egui::epaint::Primitive::Mesh(mesh) = &clipped.primitive else {
                continue;
            };
            vertex_bytes.extend_from_slice(bytemuck::cast_slice(&mesh.vertices));
            index_bytes.extend_from_slice(bytemuck::cast_slice(&mesh.indices));
            let count = mesh.indices.len() as u32;
            draws.push(Draw {
                texture: mesh.texture_id,
                indices: index_count..index_count + count,
                base_vertex: vertex_count,
                clip: clipped.clip_rect,
            });
            index_count += count;
            vertex_count += mesh.vertices.len() as i32;
        }

        if !draws.is_empty() {
            let screen = Screen {
                size_points: size.map(|n| n as f32 / pixels_per_point),
                linear: f32::from(u8::from(self.linear)),
                pad: 0.0,
            };
            queue.write_buffer(&self.screen, 0, bytemuck::bytes_of(&screen));
            self.vertices.write(device, queue, &vertex_bytes);
            self.indices.write(device, queue, &index_bytes);

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("panel"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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
            pass.set_bind_group(0, &self.screen_group, &[]);
            pass.set_vertex_buffer(0, self.vertices.buffer.slice(..));
            pass.set_index_buffer(self.indices.buffer.slice(..), wgpu::IndexFormat::Uint32);
            for draw in draws {
                let Some((_, group)) = self.textures.get(&draw.texture) else {
                    continue;
                };
                // Points to pixels, inside the target.
                let x = ((draw.clip.min.x * pixels_per_point).round().max(0.0) as u32).min(size[0]);
                let y = ((draw.clip.min.y * pixels_per_point).round().max(0.0) as u32).min(size[1]);
                let width =
                    ((draw.clip.width() * pixels_per_point).round() as u32).min(size[0] - x);
                let height =
                    ((draw.clip.height() * pixels_per_point).round() as u32).min(size[1] - y);
                if width == 0 || height == 0 {
                    continue;
                }
                pass.set_scissor_rect(x, y, width, height);
                pass.set_bind_group(1, group, &[]);
                pass.draw_indexed(draw.indices, draw.base_vertex, 0..1);
            }
        }

        for id in &textures.free {
            self.textures.remove(id);
        }
    }

    /// A whole image replaces the texture; a part is written into it.
    fn set_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) {
        let egui::ImageData::Color(image) = &delta.image;
        let size = wgpu::Extent3d {
            width: image.width() as u32,
            height: image.height() as u32,
            depth_or_array_layers: 1,
        };
        if delta.pos.is_none() || !self.textures.contains_key(&id) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("panel texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                // As written: the shader blends in egui's own space.
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("panel texture"),
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
            });
            self.textures.insert(id, (texture, group));
        }
        let (texture, _) = &self.textures[&id];
        let [x, y] = delta.pos.unwrap_or([0, 0]);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: x as u32,
                    y: y as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(image.pixels.as_slice()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: None,
            },
            size,
        );
    }
}
