//! The compositor. The world is drawn once into an HDR scene target, linear
//! light, with its depth. A chain of full screen stages follows: each reads
//! the colour and depth before it and writes the next target. The last stage
//! is always `output`: exposure, tone map and encoding, into the view's target.
//!
//! An effect is a stage: a WGSL fragment entry over [`STAGE_PRELUDE`]. Adding
//! one (bloom, grading) is a shader and a line in [`Composer::run`]. A costly
//! effect works at half size into an auxiliary target and a second stage lays
//! it over the scene at full size: the clouds do.

/// Linear light, room above white for the sun, a glint, a cloud's silver edge.
pub const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// What every stage starts with: its inputs and the screen triangle.
const STAGE_PRELUDE: &str = include_str!("shaders/stage.wgsl");

/// One full screen pass of the chain.
struct Stage {
    pipeline: wgpu::RenderPipeline,
}

impl Stage {
    fn new(
        device: &wgpu::Device,
        label: &str,
        source: &str,
        entry: &str,
        layouts: &[&wgpu::BindGroupLayout],
        format: wgpu::TextureFormat,
    ) -> Stage {
        let module = crate::shader(device, label, &format!("{STAGE_PRELUDE}\n{source}"));
        let groups: Vec<Option<&wgpu::BindGroupLayout>> =
            layouts.iter().map(|layout| Some(*layout)).collect();
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &groups,
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(entry),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
        Stage { pipeline }
    }
}

/// Which optional stages run this frame.
#[derive(Clone, Copy)]
pub struct Chain {
    pub clouds: bool,
}

pub struct Composer {
    input_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    cloud_march: Stage,
    cloud_lay: Stage,
    output: Stage,
}

impl Composer {
    /// `format` is the format of the targets the views present.
    pub fn new(
        device: &wgpu::Device,
        view_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Composer {
        let input_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stage input"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Auxiliary: what a half size stage left for the next to lay.
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("stage input"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layouts = [view_layout, &input_layout];
        Composer {
            cloud_march: Stage::new(
                device,
                "cloud march",
                include_str!("shaders/clouds.wgsl"),
                "fs_march",
                &layouts,
                SCENE_FORMAT,
            ),
            cloud_lay: Stage::new(
                device,
                "cloud lay",
                include_str!("shaders/clouds.wgsl"),
                "fs_lay",
                &layouts,
                SCENE_FORMAT,
            ),
            output: Stage::new(
                device,
                "output",
                include_str!("shaders/output.wgsl"),
                "fs",
                &layouts,
                format,
            ),
            input_layout,
            sampler,
        }
    }

    /// Runs the chain over a drawn scene and leaves the picture in `target`.
    pub fn run(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::BindGroup,
        targets: &Targets,
        chain: Chain,
        target: &wgpu::TextureView,
    ) {
        // The scene is in colour 0; each stage reads one and writes the other.
        let mut read = 0;
        // In order. Bloom enters after the clouds it should catch.
        if chain.clouds {
            // Marched at half size from the depth alone, laid at full size.
            Self::pass(
                encoder,
                &self.cloud_march,
                view,
                &targets.marching,
                &targets.half,
            );
            let write = 1 - read;
            Self::pass(
                encoder,
                &self.cloud_lay,
                view,
                &targets.inputs[read],
                &targets.color[write],
            );
            read = write;
        }
        Self::pass(encoder, &self.output, view, &targets.inputs[read], target);
    }

    fn pass(
        encoder: &mut wgpu::CommandEncoder,
        stage: &Stage,
        view: &wgpu::BindGroup,
        input: &wgpu::BindGroup,
        target: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("stage"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Every stage writes every pixel.
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
        pass.set_pipeline(&stage.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_bind_group(1, input, &[]);
        pass.draw(0..3, 0..1);
    }

    /// The targets of one view, at its size.
    pub fn targets(&self, device: &wgpu::Device, size: [u32; 2]) -> Targets {
        let texture = |label, format, usage, divide: u32| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size[0].div_ceil(divide),
                        height: size[1].div_ceil(divide),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let color = [
            texture("scene colour", SCENE_FORMAT, usage, 1),
            texture("stage colour", SCENE_FORMAT, usage, 1),
        ];
        let half = texture("half size stage", SCENE_FORMAT, usage, 2);
        let depth = texture("scene depth", crate::DEPTH_FORMAT, usage, 1);
        let input = |color: &wgpu::TextureView, aux: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("stage input"),
                layout: &self.input_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(color),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(aux),
                    },
                ],
            })
        };
        let inputs = [input(&color[0], &half), input(&color[1], &half)];
        // The half size target cannot be read while it is written.
        let marching = input(&color[0], &color[1]);
        Targets {
            size,
            color,
            half,
            depth,
            inputs,
            marching,
        }
    }
}

/// Where one view's world is drawn and its stages run.
pub struct Targets {
    pub size: [u32; 2],
    /// 0 takes the scene; the stages alternate between the two.
    pub color: [wgpu::TextureView; 2],
    pub depth: wgpu::TextureView,
    /// Where a costly stage works, half the size each way.
    half: wgpu::TextureView,
    inputs: [wgpu::BindGroup; 2],
    /// The scene's inputs for the stage that writes `half`.
    marching: wgpu::BindGroup,
}
