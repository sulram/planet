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
/// Weather turns about the planet's axis: tangent everywhere, still at the
/// poles, and a rotation never runs out of world. Metres a second at the
/// equator.
const WIND_M_S: f64 = 14.0;

/// What the shaders must agree on, stated once: prepended to every module.
pub fn prelude() -> String {
    format!(
        "const CLOUD_BASE_M: f32 = {BASE_M:?};\nconst CLOUD_TOP_M: f32 = {TOP_M:?};\n\
         const CLOUD_SHAPE_TILE_M: f32 = {SHAPE_TILE_M:?};\n\
         const CLOUD_DETAIL_TILE_M: f32 = {DETAIL_TILE_M:?};\n"
    )
}

/// How far the weather has turned at a time, as `[cos, sin]`. In f64: the
/// angle is wrapped here, where the clock still has its precision.
pub fn wind(clock_s: f64, planet_radius_m: f64) -> [f32; 2] {
    let angle = (clock_s * WIND_M_S / planet_radius_m) % core::f64::consts::TAU;
    [angle.cos() as f32, angle.sin() as f32]
}

/// Clouds also change where they stand: the field slides through the noise,
/// slowly for the bodies, faster for the wisps. Metres a second.
const BODY_DRIFT_M_S: f64 = 2.5;
const WISP_DRIFT_M_S: f64 = 9.0;
/// Metres across one repeat of the noise, for bodies and for wisps.
const SHAPE_TILE_M: f64 = 6200.0;
const DETAIL_TILE_M: f64 = 1150.0;

/// How far each has slid, as a fraction of its own repeat: wraps unseen.
pub fn drift(clock_s: f64) -> [f32; 2] {
    [
        (clock_s * BODY_DRIFT_M_S / SHAPE_TILE_M).fract() as f32,
        (clock_s * WISP_DRIFT_M_S / DETAIL_TILE_M).fract() as f32,
    ]
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

    #[test]
    fn the_wind_survives_a_long_clock() {
        // A day of play later the angle still moves by what a frame moves it.
        let (a, b) = (wind(86_400.0, 20_860.0), wind(86_400.016, 20_860.0));
        let step = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        assert!(step > 0.0 && step < 0.0001);
    }
}
