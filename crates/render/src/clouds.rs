//! Volumetric clouds: a shell of weather around the planet. This module owns
//! what a cloud is made of, a small tiling 3D noise texture drawn once on the
//! GPU, and the numbers the shaders share. The density field lives in
//! `cloud_field.wgsl` and has two readers: the `clouds` compositor stage,
//! which marches it, and every lit surface, which asks it for shade.

/// Voxels along each side of the noise texture.
const NOISE_SIZE: u32 = 64;
/// Slices along each side of the atlas they are drawn into: 8 x 8 = 64.
const ATLAS_SIDE: u32 = 8;
/// The layer, metres above the sea. The atmosphere ends at 3600.
const BASE_M: f32 = 1100.0;
const TOP_M: f32 = 3000.0;
/// What the shaders must agree on, stated once: prepended to every module.
pub fn prelude() -> String {
    format!(
        "const CLOUD_BASE_M: f32 = {BASE_M:?};\nconst CLOUD_TOP_M: f32 = {TOP_M:?};\n\
         const CLOUD_SHAPE_TILE_M: f32 = {SHAPE_TILE_M:?};\n\
         const CLOUD_DETAIL_TILE_M: f32 = {DETAIL_TILE_M:?};\n"
    )
}

/// Clouds also change where they stand: the noise rises through the layer,
/// so a heap boils and reshapes in place while the wind carries it. Slowly
/// for the bodies, faster for the wisps. Metres a second, at the usual pace.
const BODY_RISE_M_S: f64 = 9.0;
const WISP_RISE_M_S: f64 = 22.0;
/// Metres across one repeat of the noise, for bodies and for wisps.
const SHAPE_TILE_M: f64 = 6200.0;
const DETAIL_TILE_M: f64 = 1150.0;
/// The rise turns back after this many repeats. It is along the local up,
/// which no repeat of the noise lines up with, so it cannot wrap unseen; it
/// swings instead, continuous for ever and small enough to stay exact in f32.
const RISE_SWING: f64 = 64.0;
/// A clock that moved more than this between frames, or backwards, was set,
/// not run: the weather is then what that clock says from its start.
const CLOCK_JUMP_S: f64 = 5.0;

/// Where the weather has got to. Weather turns about the planet's axis:
/// tangent everywhere, still at the poles, and a rotation never runs out of
/// world. It is advanced frame by frame, in f64, so the wind can change
/// without the sky jumping; a first frame or a set clock starts it from the
/// clock alone, which keeps a headless shot a function of its clock.
#[derive(Default)]
pub struct Weather {
    clock_s: Option<f64>,
    turned_rad: f64,
    body_repeats: f64,
    wisp_repeats: f64,
}

/// What the shaders take of the weather.
pub struct WeatherNow {
    /// How far it has turned, as `[cos, sin]`.
    pub wind: [f32; 2],
    /// How far the noise has risen for bodies and wisps, in repeats of each.
    pub rise: [f32; 2],
}

impl Weather {
    pub fn advance(&mut self, clock_s: f64, effects: &scene::Effects, radius_m: f64) -> WeatherNow {
        let elapsed = match self.clock_s {
            Some(last) if (0.0..=CLOCK_JUMP_S).contains(&(clock_s - last)) => clock_s - last,
            _ => {
                *self = Weather::default();
                clock_s
            }
        };
        self.clock_s = Some(clock_s);
        let change = f64::from(effects.cloud_change);
        self.turned_rad = (self.turned_rad + elapsed * f64::from(effects.wind_m_s) / radius_m)
            % core::f64::consts::TAU;
        self.body_repeats = (self.body_repeats + elapsed * change * BODY_RISE_M_S / SHAPE_TILE_M)
            % (2.0 * RISE_SWING);
        self.wisp_repeats = (self.wisp_repeats + elapsed * change * WISP_RISE_M_S / DETAIL_TILE_M)
            % (2.0 * RISE_SWING);
        let swing = |repeats: f64| (RISE_SWING - (repeats - RISE_SWING).abs()) as f32;
        WeatherNow {
            wind: [self.turned_rad.cos() as f32, self.turned_rad.sin() as f32],
            rise: [swing(self.body_repeats), swing(self.wisp_repeats)],
        }
    }
}

pub struct Clouds {
    pub noise: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

/// One slice of the noise texture is drawn per pass; this says which.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Slice {
    /// x: depth of the slice in the texture, 0 to 1.
    at: [f32; 4],
}

impl Clouds {
    /// Draws the noise texture. Once: weather is where and when it is sampled,
    /// not what it is made of. The slices are drawn side by side into a 2D
    /// atlas and copied into the volume: a browser cannot yet be asked, through
    /// wgpu, to draw into a slice of a 3D texture.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Clouds {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud noise"),
            size: wgpu::Extent3d {
                width: NOISE_SIZE,
                height: NOISE_SIZE,
                depth_or_array_layers: NOISE_SIZE,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let noise = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud noise slice"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(size_of::<Slice>() as u64),
                },
                count: None,
            }],
        });
        let stride = device.limits().min_uniform_buffer_offset_alignment;
        let mut bytes = vec![0u8; (stride * NOISE_SIZE) as usize];
        for z in 0..NOISE_SIZE {
            let slice = Slice {
                at: [(z as f32 + 0.5) / NOISE_SIZE as f32, 0.0, 0.0, 0.0],
            };
            let start = (z * stride) as usize;
            bytes[start..start + size_of::<Slice>()].copy_from_slice(bytemuck::bytes_of(&slice));
        }
        let slices = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("cloud noise slices"),
                contents: &bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            },
        );
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud noise slice"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &slices,
                    offset: 0,
                    size: wgpu::BufferSize::new(size_of::<Slice>() as u64),
                }),
            }],
        });

        let module = crate::shader(
            device,
            "cloud noise",
            include_str!("shaders/cloud_noise.wgsl"),
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cloud noise"),
            // Group 0 is the view's everywhere else; this shader never reads it.
            bind_group_layouts: &[None, Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cloud noise"),
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
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
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

        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud noise atlas"),
            size: wgpu::Extent3d {
                width: NOISE_SIZE * ATLAS_SIDE,
                height: NOISE_SIZE * ATLAS_SIDE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let tile = |z: u32| [z % ATLAS_SIDE * NOISE_SIZE, z / ATLAS_SIDE * NOISE_SIZE];

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("cloud noise"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cloud noise slices"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &atlas_view,
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
            pass.set_pipeline(&pipeline);
            for z in 0..NOISE_SIZE {
                let [x, y] = tile(z);
                let side = NOISE_SIZE as f32;
                pass.set_viewport(x as f32, y as f32, side, side, 0.0, 1.0);
                pass.set_bind_group(1, &group, &[z * stride]);
                pass.draw(0..3, 0..1);
            }
        }
        for z in 0..NOISE_SIZE {
            let [x, y] = tile(z);
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &atlas,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: 0, y: 0, z },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: NOISE_SIZE,
                    height: NOISE_SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.submit([encoder.finish()]);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cloud noise"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Clouds { noise, sampler }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIUS_M: f64 = 20_860.0;

    #[test]
    fn a_shot_is_a_function_of_its_clock() {
        let effects = scene::Effects::default();
        let once = Weather::default().advance(730.0, &effects, RADIUS_M);
        let mut run = Weather::default();
        run.advance(100.0, &effects, RADIUS_M);
        // A set clock, not a run one: the same sky as starting there.
        let set = run.advance(730.0, &effects, RADIUS_M);
        assert_eq!((once.wind, once.rise), (set.wind, set.rise));
    }

    #[test]
    fn the_weather_never_jumps() {
        // Frame by frame across a turn of the swing, and when the wind changes.
        let mut effects = scene::Effects::default();
        let turn = RISE_SWING * DETAIL_TILE_M / WISP_RISE_M_S;
        let mut weather = Weather::default();
        let mut last = weather.advance(turn - 1.0, &effects, RADIUS_M);
        for frame in 1..120 {
            if frame == 60 {
                effects.wind_m_s = 60.0;
            }
            let now = weather.advance(turn - 1.0 + f64::from(frame) / 60.0, &effects, RADIUS_M);
            assert!((now.rise[1] - last.rise[1]).abs() < 0.001);
            assert!((now.wind[0] - last.wind[0]).abs() < 0.0001);
            last = now;
        }
    }
}
