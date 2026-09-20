//! The renderer. All of wgpu lives here.
//!
//! It draws a [`scene::Frame`]: terrain patches, boxes, sky. It knows nothing
//! about recipes, addresses or input. Rules it keeps (CLAUDE.md invariants):
//! - **camera-relative**: the GPU never sees a planet space position. Every
//!   `f64` origin is subtracted from the camera on the CPU, per frame.
//! - **reversed infinite depth**: `Depth32Float`, near is 1, far is 0.
//! - **N views**: every per view resource lives in [`ViewResources`]; desktop
//!   passes one [`View`], a headset will pass two.

mod boxes;
mod gpu;
#[cfg(not(target_arch = "wasm32"))]
mod headless;
mod skinned;
mod terrain;

use bytemuck::{Pod, Zeroable};
use glam::{DVec3, Mat4, Vec3};
use scene::{Camera, Frame, SkinnedChange, TerrainChange};

pub use gpu::{Gpu, surface_configuration};
#[cfg(not(target_arch = "wasm32"))]
pub use headless::{Headless, write_png};
pub use wgpu;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Top of the atmosphere above sea level, metres.
const ATMOSPHERE_M: f64 = 2500.0;
/// Angular radius of the moon in the sky. Three times ours: it is a small sky.
const MOON_RADIUS_RAD: f32 = 0.014;

/// One camera drawing into one target.
pub struct View<'a> {
    pub camera: Camera,
    pub target: &'a wgpu::TextureView,
    pub size: [u32; 2],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct ViewUniform {
    clip_from_relative: [[f32; 4]; 4],
    relative_from_clip: [[f32; 4]; 4],
    camera: [f32; 4],
    sun: [f32; 4],
    moon: [f32; 4],
    flags: [f32; 4],
}

/// GPU state owned by one view slot.
struct ViewResources {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth: Option<(wgpu::TextureView, [u32; 2])>,
    patches: terrain::PatchUniforms,
    boxes: boxes::Instances,
    skinned: skinned::InstanceUniforms,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    encode_srgb: bool,
    view_layout: wgpu::BindGroupLayout,
    views: Vec<ViewResources>,
    terrain: terrain::Terrain,
    boxes: boxes::Boxes,
    skinned: skinned::Skinned,
    sky: wgpu::RenderPipeline,
}

impl Renderer {
    /// `format` is the format of the targets this renderer will draw into.
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Renderer {
        let device = gpu.device.clone();
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view"),
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
        let terrain = terrain::Terrain::new(&device, &view_layout, format);
        let boxes = boxes::Boxes::new(&device, &view_layout, format);
        let skinned = skinned::Skinned::new(&device, &view_layout, format);
        let sky = sky_pipeline(&device, &view_layout, format);
        Renderer {
            device,
            queue: gpu.queue.clone(),
            encode_srgb: !format.is_srgb(),
            view_layout,
            views: Vec::new(),
            terrain,
            boxes,
            skinned,
            sky,
        }
    }

    /// Uploads and drops patch meshes, in the order the client produced them.
    pub fn apply(&mut self, changes: Vec<TerrainChange>) {
        for change in changes {
            self.terrain.apply(&self.device, change);
        }
    }

    /// Uploads and drops skinned meshes (avatars).
    pub fn apply_skinned(&mut self, changes: Vec<SkinnedChange>) {
        for change in changes {
            self.skinned.apply(&self.device, &self.queue, change);
        }
    }

    /// Draws the frame once per view and submits.
    pub fn render(&mut self, frame: &Frame, views: &[View<'_>]) {
        while self.views.len() < views.len() {
            let resources = self.view_resources();
            self.views.push(resources);
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        for (index, view) in views.iter().enumerate() {
            self.render_view(&mut encoder, frame, view, index);
        }
        self.queue.submit([encoder.finish()]);
    }

    fn render_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        view: &View<'_>,
        index: usize,
    ) {
        let [width, height] = view.size.map(|n| n.max(1));
        let uniform = self.view_uniform(frame, &view.camera, width as f32 / height as f32);

        let resources = &mut self.views[index];
        self.queue
            .write_buffer(&resources.uniform, 0, bytemuck::bytes_of(&uniform));
        let drawn = resources.patches.write(
            &self.device,
            &self.queue,
            &self.terrain,
            &frame.patches,
            view.camera.position,
        );
        let box_count = resources.boxes.write(
            &self.device,
            &self.queue,
            &frame.boxes,
            view.camera.position,
        );
        let skinned_drawn = resources.skinned.write(
            &self.device,
            &self.queue,
            &self.skinned,
            &frame.skinned,
            view.camera.position,
        );

        if resources
            .depth
            .as_ref()
            .is_none_or(|(_, size)| *size != [width, height])
        {
            resources.depth = Some((depth_texture(&self.device, width, height), [width, height]));
        }
        let depth = &resources.depth.as_ref().expect("just ensured").0;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: view.target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    // Reversed depth: far is 0.
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &resources.bind_group, &[]);
        self.terrain.draw(&mut pass, &resources.patches, &drawn);
        self.boxes.draw(&mut pass, &resources.boxes, box_count);
        self.skinned
            .draw(&mut pass, &resources.skinned, &skinned_drawn);
        pass.set_pipeline(&self.sky);
        pass.draw(0..3, 0..1);
        // Last: the sea blends over the world and the sky behind it.
        self.terrain
            .draw_water(&mut pass, &resources.patches, &drawn);
    }

    fn view_uniform(&self, frame: &Frame, camera: &Camera, aspect: f32) -> ViewUniform {
        let projection = Mat4::perspective_infinite_reverse_rh(camera.fov_y, aspect, camera.near);
        let view_from_relative = Mat4::from_quat(camera.rotation.inverse().as_quat());
        let clip_from_relative = projection * view_from_relative;
        let radius = frame.planet_radius_m;
        ViewUniform {
            clip_from_relative: clip_from_relative.to_cols_array_2d(),
            relative_from_clip: clip_from_relative.inverse().to_cols_array_2d(),
            camera: camera.position.as_vec3().extend(radius as f32).to_array(),
            sun: frame
                .sun_direction
                .extend((radius + ATMOSPHERE_M) as f32)
                .to_array(),
            moon: frame.moon_direction.extend(MOON_RADIUS_RAD).to_array(),
            flags: [
                f32::from(u8::from(self.encode_srgb)),
                // Wrapped so f32 keeps sub millisecond steps all day.
                (frame.clock_s % 3600.0) as f32,
                (camera.position.length() - radius) as f32,
                0.0,
            ],
        }
    }

    fn view_resources(&self) -> ViewResources {
        let uniform = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &self.view_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        ViewResources {
            uniform,
            bind_group,
            depth: None,
            patches: terrain::PatchUniforms::new(&self.device, self.terrain.patch_layout()),
            boxes: boxes::Instances::new(&self.device),
            skinned: skinned::InstanceUniforms::default(),
        }
    }
}

/// `origin - camera`, the one subtraction that keeps `f32` honest.
fn relative(origin: DVec3, camera: DVec3) -> Vec3 {
    (origin - camera).as_vec3()
}

fn depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// WGSL with the shared prelude in front.
fn shader(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    let source = format!("{}\n{source}", include_str!("shaders/common.wgsl"));
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

/// How a pipeline treats depth, faces and blending.
#[derive(Clone, Copy, PartialEq)]
enum Surface {
    /// Opaque geometry: writes depth, culls back faces.
    Solid,
    /// The sky: no depth write, no culling.
    Backdrop,
    /// Water: blended over what is there, seen from both sides.
    Translucent,
}

/// What differs between our pipelines; the rest is fixed in [`pipeline`].
struct PipelineSpec<'a> {
    label: &'a str,
    source: &'a str,
    layouts: &'a [&'a wgpu::BindGroupLayout],
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    surface: Surface,
}

fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    spec: PipelineSpec<'_>,
) -> wgpu::RenderPipeline {
    let module = shader(device, spec.label, spec.source);
    let groups: Vec<Option<&wgpu::BindGroupLayout>> =
        spec.layouts.iter().map(|layout| Some(*layout)).collect();
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(spec.label),
        bind_group_layouts: &groups,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            buffers: spec.buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: (spec.surface == Surface::Translucent)
                    .then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: (spec.surface == Surface::Solid).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(spec.surface == Surface::Solid),
            // Reversed depth: nearer is greater. The sky sits at exactly 0.
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn sky_pipeline(
    device: &wgpu::Device,
    view_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    pipeline(
        device,
        format,
        PipelineSpec {
            label: "sky",
            source: include_str!("shaders/sky.wgsl"),
            layouts: &[view_layout],
            buffers: &[],
            surface: Surface::Backdrop,
        },
    )
}
