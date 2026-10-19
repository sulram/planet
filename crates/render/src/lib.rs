//! The renderer. All of wgpu lives here.
//!
//! It draws a [`scene::Frame`]: terrain patches, volumes, boxes, pictures, portal panes, sky. It knows nothing
//! about recipes, addresses or input. Rules it keeps (CLAUDE.md invariants):
//! - **camera-relative**: the GPU never sees a planet space position. Every
//!   `f64` origin is subtracted from the camera on the CPU, per frame.
//! - **reversed infinite depth**: `Depth32Float`, near is 1, far is 0.
//! - **N views**: every per view resource lives in [`ViewResources`]; desktop
//!   passes one [`View`], a headset will pass two. The world past a portal
//!   is a sight of its own, drawn before them into a slot of its own.

mod boxes;
mod clouds;
mod compose;
mod gpu;
#[cfg(not(target_arch = "wasm32"))]
mod headless;
mod models;
mod panes;
mod pictures;
mod shadow;
mod skinned;
mod terrain;
mod volumes;

use bytemuck::{Pod, Zeroable};
use glam::{DVec3, Mat4, Vec3};
use scene::{
    Camera, Frame, ModelChange, PatchDraw, PictureChange, SkinnedChange, TerrainChange,
    VolumeChange,
};

pub use gpu::{Gpu, surface_configuration};
#[cfg(not(target_arch = "wasm32"))]
pub use headless::{HEADLESS_FORMAT, Headless, Over, write_png};
pub use wgpu;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Top of the atmosphere above sea level, metres.
const ATMOSPHERE_M: f64 = 3600.0;
/// What one draw of the world sees: a camera, the patches in its view, and
/// whether the whole world is drawn, or the ground, the sea, the sky and
/// the cells alone, as the world past a portal is: what is built there
/// shows through the pane, the parts and the bodies do not.
struct Sight<'a> {
    camera: Camera,
    size: [u32; 2],
    patches: &'a [PatchDraw],
    whole: bool,
}

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
    /// The passage's warp: `xy` its point on the screen, `z` its phase, `w`
    /// 1 while there is one.
    warp: [f32; 4],
    lamps_on: [f32; 4],
    lamps: [[[f32; 4]; 2]; volumes::LAMPS],
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
    pictures: pictures::Instances,
    panes: panes::Instances,
    volumes: terrain::PatchUniforms,
    models: models::InstanceUniforms,
}

pub struct Renderer {
    weather: clouds::Weather,
    device: wgpu::Device,
    queue: wgpu::Queue,
    encode_srgb: bool,
    view_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    views: Vec<ViewResources>,
    /// The slot the world past a portal is drawn in, once one has been.
    through: Option<ViewResources>,
    terrain: terrain::Terrain,
    volumes: volumes::Volumes,
    boxes: boxes::Boxes,
    skinned: skinned::Skinned,
    pictures: pictures::Pictures,
    panes: panes::Panes,
    models: models::Models,
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
        let pictures =
            pictures::Pictures::new(&device, &gpu.queue, &view_layout, &shadow_layout, scene);
        let panes = panes::Panes::new(
            &device,
            &gpu.queue,
            &view_layout,
            composer.behind_layout(),
            scene,
        );
        let models = models::Models::new(&device, &view_layout, &shadow_layout, scene);
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
            through: None,
            terrain,
            volumes,
            boxes,
            skinned,
            pictures,
            panes,
            models,
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

    /// Uploads and drops pictures, each with its mip chain.
    pub fn apply_pictures(&mut self, changes: Vec<PictureChange>) {
        for change in changes {
            self.pictures.apply(&self.device, &self.queue, change);
        }
    }

    /// Uploads and drops the meshes of models (parts).
    pub fn apply_models(&mut self, changes: Vec<ModelChange>) {
        for change in changes {
            self.models.apply(&self.device, change);
        }
    }

    /// Draws the frame once per view and submits. The world past a portal
    /// is drawn first, from the eye carried through it, at a quarter of the
    /// first view's height: every view's panes sample it.
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
        // The views' resources step out of `self` while the subsystems draw
        // into them.
        let mut slots = core::mem::take(&mut self.views);
        let mut through_slot = self.through.take();
        let through = frame
            .through
            .as_ref()
            .zip(views.first())
            .map(|(through, view)| {
                let resources = through_slot.get_or_insert_with(|| self.view_resources());
                let sight = Sight {
                    camera: through.camera,
                    size: view
                        .size
                        .map(|n| ((f32::from(n as u16) * through.size).ceil() as u32).max(1)),
                    patches: &through.patches,
                    whole: false,
                };
                let at = self.draw_world(&mut encoder, resources, frame, &weather, &sight, None);
                let targets = resources.targets.as_ref().expect("drawn");
                self.panes
                    .through_group(&self.device, self.composer.picture(targets, &at))
            });
        for (index, view) in views.iter().enumerate() {
            let sight = Sight {
                camera: view.camera,
                size: view.size,
                patches: &frame.patches,
                whole: true,
            };
            let resources = &mut slots[index];
            let mut at = self.draw_world(
                &mut encoder,
                resources,
                frame,
                &weather,
                &sight,
                through.as_ref(),
            );
            let targets = resources.targets.as_ref().expect("drawn");
            // A body's passage through a portal draws the picture into it
            // and out again, before the picture is finished: the bloom
            // takes the hole's rim.
            if frame.warp.is_some() {
                self.composer
                    .warp(&mut encoder, &resources.bind_group, targets, &mut at);
            }
            self.composer.finish(
                &mut encoder,
                &resources.bind_group,
                targets,
                at,
                frame.effects.bloom > 0.0,
                view.target,
            );
        }
        self.views = slots;
        self.through = through_slot;
        self.queue.submit([encoder.finish()]);
    }

    /// Draws the world one sight sees into its resources' targets, up to
    /// the stages that finish a picture: the sun's shadows, the opaque
    /// world and the sky, then the sea and the clouds, then what goes over
    /// them. Returns where the picture so far is.
    fn draw_world(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut ViewResources,
        frame: &Frame,
        weather: &clouds::WeatherNow,
        sight: &Sight<'_>,
        through: Option<&wgpu::BindGroup>,
    ) -> compose::Cursor {
        let [width, height] = sight.size.map(|n| n.max(1));
        let camera = &sight.camera;
        let shadows = sight.whole && frame.effects.shadows;
        let uniform = self.view_uniform(
            frame,
            weather,
            camera,
            width as f32 / height as f32,
            shadows,
        );
        self.queue
            .write_buffer(&resources.uniform, 0, bytemuck::bytes_of(&uniform));
        let drawn = resources.patches.write(
            &self.device,
            &self.queue,
            &self.terrain,
            sight.patches,
            camera.position,
        );
        // What stands on the ground is the whole world's: the world past a
        // portal is its ground, sea and sky alone.
        // The cells are every sight's; the rest of what is built the whole
        // world's.
        let volumes = self.volumes.place(
            &self.device,
            &self.queue,
            &mut resources.volumes,
            frame,
            camera.position,
        );
        let placed = sight.whole.then(|| {
            let box_count =
                resources
                    .boxes
                    .write(&self.device, &self.queue, &frame.boxes, camera.position);
            let skinned = resources.skinned.write(
                &self.device,
                &self.queue,
                &self.skinned,
                &frame.skinned,
                camera.position,
            );
            resources
                .pictures
                .write(&self.device, &self.queue, &frame.pictures, camera.position);
            resources
                .panes
                .write(&self.device, &self.queue, &frame.panes, camera.position);
            let models = resources.models.write(
                &self.device,
                &self.queue,
                &self.models,
                &frame.models,
                camera.position,
            );
            (box_count, skinned, models)
        });

        if shadows {
            let matrices = shadow::matrices(frame, camera);
            let wanted: Vec<_> = frame
                .shadow_patches
                .iter()
                .copied()
                .filter(|draw| self.terrain.casts_into(draw, camera.position, &matrices))
                .collect();
            let casters = resources.casters.write(
                &self.device,
                &self.queue,
                &self.terrain,
                &wanted,
                camera.position,
            );
            let (box_count, skinned, models) = placed.as_ref().expect("whole");
            for (cascade, matrix) in matrices.iter().enumerate() {
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
                    .draw_shadow(&mut pass, &resources.boxes, *box_count);
                self.skinned
                    .draw_shadow(&mut pass, &resources.skinned, skinned);
                self.pictures
                    .draw_shadow(&mut pass, &resources.pictures, &frame.pictures);
                self.models
                    .draw_shadow(&mut pass, &resources.models, models, *matrix);
            }
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
        self.volumes.draw(&mut pass, &resources.volumes, &volumes);
        if let Some((box_count, skinned, models)) = &placed {
            if frame.effects.grass {
                self.terrain
                    .draw_grass(&mut pass, &resources.patches, &drawn);
            }
            self.models.draw(&mut pass, &resources.models, models);
            self.boxes.draw(&mut pass, &resources.boxes, *box_count);
            self.skinned.draw(&mut pass, &resources.skinned, skinned);
            self.pictures
                .draw(&mut pass, &resources.pictures, &frame.pictures);
        }
        pass.set_pipeline(&self.sky);
        pass.draw(0..3, 0..1);
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
        // The portals' panes go over the sea and the clouds, tested against
        // the depth of what is solid: a lens bends a copy of what is behind
        // it, a window shows the world past the portal.
        if sight.whole && !frame.panes.is_empty() {
            let (over, behind) = self.composer.behind(encoder, targets, &at);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("panes"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: over,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                // The depth is read only here: the lens reads it as a
                // texture in the same pass, and a pane writes none.
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &resources.bind_group, &[]);
            pass.set_bind_group(2, behind, &[]);
            self.panes
                .draw(&mut pass, &resources.panes, &frame.panes, through);
        }
        // Glass, and what only a builder sees, go over the sea and the
        // clouds, tested against the depth of what is solid: drawn before
        // them, the sea behind a pane would be laid over it.
        if let Some((_, _, models)) = &placed
            && (volumes.over() || models.over())
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("over"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.composer.picture(targets, &at),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &resources.bind_group, &[]);
            self.volumes
                .draw_over(&mut pass, &resources.volumes, &volumes);
            self.models.draw_over(&mut pass, &resources.models, models);
        }
        at
    }

    fn view_uniform(
        &self,
        frame: &Frame,
        weather: &clouds::WeatherNow,
        camera: &Camera,
        aspect: f32,
        shadows: bool,
    ) -> ViewUniform {
        let projection = Mat4::perspective_infinite_reverse_rh(camera.fov_y, aspect, camera.near);
        let view_from_relative = Mat4::from_quat(camera.rotation.inverse().as_quat());
        let clip_from_relative = projection * view_from_relative;
        let radius = frame.planet_radius_m;
        let exposure = frame.effects.exposure;
        let (lamps, lamps_on) = volumes::lamps(&frame.lamps, camera.position);
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
            warp: frame.warp.map_or([0.0; 4], |warp| {
                [warp.centre[0], warp.centre[1], warp.phase, 1.0]
            }),
            lamps_on: [lamps_on, 0.0, 0.0, 0.0],
            lamps,
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
                f32::from(u8::from(shadows)),
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
            pictures: pictures::Instances::new(&self.device),
            panes: panes::Instances::new(&self.device),
            volumes: terrain::PatchUniforms::new(&self.device, self.terrain.patch_layout()),
            models: models::InstanceUniforms::default(),
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
        "{}{}{}{}{}\n{}\n{source}",
        shadow::prelude(),
        clouds::prelude(),
        compose::prelude(),
        volumes::prelude(),
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
    /// Glass: blended over what is there, tested against depth and writing
    /// none. Its fragment entry is `fs_glass`.
    Glass,
    /// The ghost of a stroke over the world: blended, tested against depth
    /// and writing none.
    Ghost,
    /// A guide over the world: as a ghost is, and seen from both sides.
    Guide,
    /// A mark over everything: tested against no depth and writing none.
    Overlay,
    /// A portal's pane: opaque, tested against depth and writing none,
    /// drawn after the sea and the clouds over a copy of what is behind it.
    Lens,
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
            Surface::Translucent | Surface::Glass | Surface::Ghost | Surface::Guide
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
                Surface::Glass => "fs_glass",
                _ => "fs",
            }),
            targets: if shadow { &[] } else { &targets },
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: (matches!(
                spec.surface,
                Surface::Solid | Surface::Glass | Surface::Ghost | Surface::Overlay | Surface::Lens
            ) || shadow)
                .then_some(wgpu::Face::Back),
            ..Default::default()
        },
        // The sea reads the depth, so it cannot also be tested against it.
        depth_stencil: (spec.surface != Surface::Translucent).then_some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(
                spec.surface == Surface::Solid || spec.surface == Surface::Foliage || shadow,
            ),
            // Reversed depth: nearer is greater. The sky sits at exactly 0. A
            // mark over everything passes whatever is there.
            depth_compare: Some(match spec.surface {
                Surface::Overlay => wgpu::CompareFunction::Always,
                _ => wgpu::CompareFunction::GreaterEqual,
            }),
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
