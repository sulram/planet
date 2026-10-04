//! The renderer. All of wgpu lives here.
//!
//! It draws a [`scene::Frame`]: terrain patches, volumes, boxes, sky. It knows nothing
//! about recipes, addresses or input. Rules it keeps (CLAUDE.md invariants):
//! - **camera-relative**: the GPU never sees a planet space position. Every
//!   `f64` origin is subtracted from the camera on the CPU, per frame.
//! - **reversed infinite depth**: `Depth32Float`, near is 1, far is 0.
//! - **N views**: every per view resource lives in [`ViewResources`]; desktop
//!   passes one [`View`], a headset will pass two.

mod boxes;
mod clouds;
mod compose;
mod gpu;
#[cfg(not(target_arch = "wasm32"))]
mod headless;
mod shadow;
mod skinned;
mod terrain;
mod volumes;

use bytemuck::{Pod, Zeroable};
use glam::{DVec3, Mat4, Vec3};
use scene::{Camera, Frame, SkinnedChange, TerrainChange, VolumeChange};

pub use gpu::{Gpu, surface_configuration};
#[cfg(not(target_arch = "wasm32"))]
pub use headless::{HEADLESS_FORMAT, Headless, Over, write_png};
pub use wgpu;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Top of the atmosphere above sea level, metres.
const ATMOSPHERE_M: f64 = 3600.0;
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
    moon_light: [f32; 4],
    flags: [f32; 4],
    shadow_clip: [[[f32; 4]; 4]; shadow::CASCADES],
    shadow_texel_m: [f32; 4],
    interaction_start: [f32; 4],
    interaction_end: [f32; 4],
    post: [f32; 4],
    clouds: [f32; 4],
    bloom: [f32; 4],
    grade: [f32; 4],
}

/// GPU state owned by one view slot.
struct ViewResources {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    targets: Option<compose::Targets>,
    patches: terrain::PatchUniforms,
    casters: terrain::PatchUniforms,
    shadows: shadow::Maps,
    boxes: boxes::Instances,
    skinned: skinned::InstanceUniforms,
    volumes: terrain::PatchUniforms,
}

pub struct Renderer {
    weather: clouds::Weather,
    device: wgpu::Device,
    queue: wgpu::Queue,
    encode_srgb: bool,
    view_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    views: Vec<ViewResources>,
    terrain: terrain::Terrain,
    volumes: volumes::Volumes,
    boxes: boxes::Boxes,
    skinned: skinned::Skinned,
    sky: wgpu::RenderPipeline,
    composer: compose::Composer,
    clouds: clouds::Clouds,
}

impl Renderer {
    /// `format` is the format of the targets this renderer will present into.
    /// The world itself is drawn in [`compose::SCENE_FORMAT`].
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
        let shadow_layout = view_layout;
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view and sun shadows"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let scene = compose::SCENE_FORMAT;
        let composer = compose::Composer::new(&device, &view_layout, format);
        let terrain = terrain::Terrain::new(
            &device,
            &view_layout,
            &shadow_layout,
            composer.behind_layout(),
            scene,
        );
        let volumes = volumes::Volumes::new(
            &device,
            &view_layout,
            &shadow_layout,
            terrain.patch_layout(),
            scene,
        );
        let boxes = boxes::Boxes::new(&device, &view_layout, &shadow_layout, scene);
        let skinned = skinned::Skinned::new(&device, &view_layout, &shadow_layout, scene);
        let sky = sky_pipeline(&device, &view_layout, scene);
        let clouds = clouds::Clouds::new(&device, &gpu.queue);
        Renderer {
            weather: clouds::Weather::default(),
            device,
            queue: gpu.queue.clone(),
            encode_srgb: !format.is_srgb(),
            view_layout,
            shadow_layout,
            views: Vec::new(),
            terrain,
            volumes,
            boxes,
            skinned,
            sky,
            composer,
            clouds,
        }
    }

    /// Uploads and drops patch meshes, in the order the client produced them.
    pub fn apply(&mut self, changes: Vec<TerrainChange>) {
        for change in changes {
            self.terrain.apply(&self.device, change);
        }
    }

    /// Uploads and drops the meshes of volumes, of the ghost of a stroke and
    /// of guides.
    pub fn apply_volumes(&mut self, changes: Vec<VolumeChange>) {
        for change in changes {
            self.volumes.apply(&self.device, change);
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
        // Once a frame, not once a view: both eyes see one sky.
        let weather = self
            .weather
            .advance(frame.clock_s, &frame.effects, frame.planet_radius_m);
        for (index, view) in views.iter().enumerate() {
            self.render_view(&mut encoder, frame, &weather, view, index);
        }
        self.queue.submit([encoder.finish()]);
    }

    fn render_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        weather: &clouds::WeatherNow,
        view: &View<'_>,
        index: usize,
    ) {
        let [width, height] = view.size.map(|n| n.max(1));
        let uniform = self.view_uniform(frame, weather, &view.camera, width as f32 / height as f32);

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
        let volumes = self.volumes.place(
            &self.device,
            &self.queue,
            &mut resources.volumes,
            frame,
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

        let matrices = shadow::matrices(frame, &view.camera);
        let wanted: Vec<_> = frame
            .shadow_patches
            .iter()
            .copied()
            .filter(|draw| {
                frame.effects.shadows
                    && self
                        .terrain
                        .casts_into(draw, view.camera.position, &matrices)
            })
            .collect();
        let casters = resources.casters.write(
            &self.device,
            &self.queue,
            &self.terrain,
            &wanted,
            view.camera.position,
        );
        for (cascade, matrix) in matrices
            .iter()
            .enumerate()
            .filter(|_| frame.effects.shadows)
        {
            let mut light = uniform;
            light.clip_from_relative = matrix.to_cols_array_2d();
            self.queue.write_buffer(
                &resources.shadows.uniforms[cascade],
                0,
                bytemuck::bytes_of(&light),
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sun shadows"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &resources.shadows.layers[cascade],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &resources.shadows.groups[cascade], &[]);
            self.terrain
                .draw_shadow(&mut pass, &resources.casters, &casters, *matrix);
            self.volumes
                .draw_shadow(&mut pass, &resources.volumes, &volumes, *matrix);
            self.boxes
                .draw_shadow(&mut pass, &resources.boxes, box_count);
            self.skinned
                .draw_shadow(&mut pass, &resources.skinned, &skinned_drawn);
        }

        if resources
            .targets
            .as_ref()
            .is_none_or(|targets| targets.size != [width, height])
        {
            resources.targets = Some(self.composer.targets(&self.device, [width, height]));
        }
        let targets = resources.targets.as_ref().expect("just ensured");

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.color[0],
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations {
                    // Reversed depth: far is 0. Kept: the stages read it.
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &resources.bind_group, &[]);
        self.terrain.draw(&mut pass, &resources.patches, &drawn);
        if frame.effects.grass {
            self.terrain
                .draw_grass(&mut pass, &resources.patches, &drawn);
        }
        self.volumes.draw(&mut pass, &resources.volumes, &volumes);
        self.boxes.draw(&mut pass, &resources.boxes, box_count);
        self.skinned
            .draw(&mut pass, &resources.skinned, &skinned_drawn);
        pass.set_pipeline(&self.sky);
        pass.draw(0..3, 0..1);
        self.volumes
            .draw_over(&mut pass, &resources.volumes, &volumes);
        drop(pass);

        // Then the sea and the clouds, the nearer last: a camera under the
        // sea sees the clouds through its surface, any other sees them over it.
        let mut at = compose::Cursor::default();
        let submerged = uniform.flags[2] < 0.0;
        if frame.effects.clouds && submerged {
            self.composer
                .clouds(encoder, &resources.bind_group, targets, &mut at);
        }
        // The sea reads a copy of the picture so far and the depth, to refract
        // and absorb what lies behind it, so it tests depth itself: a texture
        // cannot be read while it is written.
        let (over, behind) = self.composer.behind(encoder, targets, &at);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sea"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: over,
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
        pass.set_bind_group(0, &resources.bind_group, &[]);
        pass.set_bind_group(2, behind, &[]);
        self.terrain
            .draw_water(&mut pass, &resources.patches, &drawn);
        drop(pass);
        if frame.effects.clouds && !submerged {
            self.composer
                .clouds(encoder, &resources.bind_group, targets, &mut at);
        }
        self.composer.finish(
            encoder,
            &resources.bind_group,
            targets,
            at,
            frame.effects.bloom > 0.0,
            view.target,
        );
    }

    fn view_uniform(
        &self,
        frame: &Frame,
        weather: &clouds::WeatherNow,
        camera: &Camera,
        aspect: f32,
    ) -> ViewUniform {
        let projection = Mat4::perspective_infinite_reverse_rh(camera.fov_y, aspect, camera.near);
        let view_from_relative = Mat4::from_quat(camera.rotation.inverse().as_quat());
        let clip_from_relative = projection * view_from_relative;
        let radius = frame.planet_radius_m;
        let exposure = frame.effects.exposure;
        ViewUniform {
            clip_from_relative: clip_from_relative.to_cols_array_2d(),
            relative_from_clip: clip_from_relative.inverse().to_cols_array_2d(),
            camera: camera.position.as_vec3().extend(radius as f32).to_array(),
            sun: frame
                .sun_direction
                .extend((radius + ATMOSPHERE_M) as f32)
                .to_array(),
            // Centre relative to the camera, in f64 first: it is far away.
            moon: relative(frame.moon.position, camera.position)
                .extend(frame.moon.radius_m as f32)
                .to_array(),
            moon_light: frame
                .moon
                .position
                .normalize()
                .as_vec3()
                .extend(0.0)
                .to_array(),
            shadow_clip: shadow::matrices(frame, camera).map(|m| m.to_cols_array_2d()),
            shadow_texel_m: shadow::texels_m(),
            interaction_start: relative(frame.interaction.start, camera.position)
                .extend(frame.interaction.radius_m)
                .to_array(),
            interaction_end: relative(frame.interaction.end, camera.position)
                .extend(0.0)
                .to_array(),
            post: {
                let [body, wisp] = weather.rise;
                [exposure, body, wisp, frame.effects.cloud_density]
            },
            grade: [
                frame.effects.haze,
                frame.effects.tone_map.index() as f32,
                frame.effects.water_clarity,
                0.0,
            ],
            bloom: {
                // The threshold is of exposed light, as in a lens.
                let threshold = frame.effects.bloom_threshold / exposure;
                // Half the threshold: light eases into glowing, never pops.
                [frame.effects.bloom, threshold, threshold * 0.5, 0.0]
            },
            clouds: {
                let [cos, sin] = weather.wind;
                let on = f32::from(u8::from(frame.effects.clouds));
                [cos, sin, frame.effects.cloud_cover, on]
            },
            flags: [
                f32::from(u8::from(self.encode_srgb)),
                // Wrapped so f32 keeps sub millisecond steps all day.
                (frame.clock_s % 3600.0) as f32,
                (camera.position.length() - radius) as f32,
                f32::from(u8::from(frame.effects.shadows)),
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
        let shadows = shadow::Maps::new(&self.device, &self.shadow_layout);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &self.view_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadows.sampled),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadows.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.clouds.noise),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.clouds.sampler),
                },
            ],
        });
        ViewResources {
            uniform,
            bind_group,
            targets: None,
            shadows,
            casters: terrain::PatchUniforms::new(&self.device, self.terrain.patch_layout()),
            patches: terrain::PatchUniforms::new(&self.device, self.terrain.patch_layout()),
            boxes: boxes::Instances::new(&self.device),
            skinned: skinned::InstanceUniforms::default(),
            volumes: terrain::PatchUniforms::new(&self.device, self.terrain.patch_layout()),
        }
    }
}

/// `origin - camera`, the one subtraction that keeps `f32` honest.
fn relative(origin: DVec3, camera: DVec3) -> Vec3 {
    (origin - camera).as_vec3()
}

/// WGSL with the shared prelude in front.
fn shader(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    let source = format!(
        "{}{}{}{}\n{}\n{source}",
        shadow::prelude(),
        clouds::prelude(),
        compose::prelude(),
        include_str!("shaders/common.wgsl"),
        include_str!("shaders/cloud_field.wgsl")
    );
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
    /// A preview over the world: blended, tested against depth and writing
    /// none. Its fragment entry is `fs_ghost`.
    Ghost,
    /// A guide over the world: as a ghost is, and seen from both sides.
    Guide,
    Foliage,
    Shadow,
    ShadowCutout,
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
    let shadow = matches!(spec.surface, Surface::Shadow | Surface::ShadowCutout);
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend: matches!(
            spec.surface,
            Surface::Translucent | Surface::Ghost | Surface::Guide
        )
        .then_some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
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
        fragment: (spec.surface != Surface::Shadow).then_some(wgpu::FragmentState {
            module: &module,
            entry_point: Some(match spec.surface {
                _ if shadow => "fs_shadow",
                Surface::Ghost => "fs_ghost",
                _ => "fs",
            }),
            targets: if shadow { &[] } else { &targets },
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: (matches!(spec.surface, Surface::Solid | Surface::Ghost) || shadow)
                .then_some(wgpu::Face::Back),
            ..Default::default()
        },
        // The sea reads the depth, so it cannot also be tested against it.
        depth_stencil: (spec.surface != Surface::Translucent).then_some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(
                spec.surface == Surface::Solid || spec.surface == Surface::Foliage || shadow,
            ),
            // Reversed depth: nearer is greater. The sky sits at exactly 0.
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: Default::default(),
            bias: if shadow {
                wgpu::DepthBiasState {
                    constant: -2,
                    slope_scale: -1.5,
                    clamp: 0.0,
                }
            } else {
                Default::default()
            },
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
