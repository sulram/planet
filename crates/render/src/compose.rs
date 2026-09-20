//! The compositor. The world is drawn once into an HDR scene target, linear
//! light, with its depth. A chain of full screen stages follows: each reads
//! the colour and depth before it and writes the next target. The last stage
//! is always `output`: exposure, tone map and encoding, into the view's target.
//!
//! The sea is drawn between the two: the opaque world is copied aside, and the
//! water reads that copy and the depth to refract and absorb what lies behind
//! it ([`Composer::behind`]).
//!
//! An effect is a stage: a WGSL fragment entry over [`STAGE_PRELUDE`]. Adding
//! one (bloom, grading) is a shader and a line in [`Composer::run`]. A costly
//! effect works at half size into an auxiliary target and a second stage lays
//! it over the scene at full size: the clouds do.

/// Linear light, room above white for the sun, a glint, a cloud's silver edge.
pub const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// What every stage starts with: its inputs and the screen triangle.
const STAGE_PRELUDE: &str = include_str!("shaders/stage.wgsl");

/// How a stage leaves its target: most write every pixel, some add light to
/// what another stage left there.
#[derive(Clone, Copy, PartialEq)]
enum Write {
    Replace,
    Add,
}

const ADD: wgpu::BlendComponent = wgpu::BlendComponent {
    src_factor: wgpu::BlendFactor::One,
    dst_factor: wgpu::BlendFactor::One,
    operation: wgpu::BlendOperation::Add,
};

/// Halvings in the bloom pyramid: the widest glow is 2^6 pixels of the scene.
const BLOOM_LEVELS: usize = 5;

/// What the shaders must agree on, stated once: prepended to every module.
pub fn prelude() -> String {
    format!("const BLOOM_LEVELS: u32 = {BLOOM_LEVELS}u;\n")
}

/// One full screen pass of the chain.
struct Stage {
    pipeline: wgpu::RenderPipeline,
    write: Write,
}

impl Stage {
    fn new(
        device: &wgpu::Device,
        label: &str,
        source: &str,
        entry: &str,
        layouts: &[&wgpu::BindGroupLayout],
        format: wgpu::TextureFormat,
        write: Write,
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
                    blend: (write == Write::Add).then_some(wgpu::BlendState {
                        color: ADD,
                        alpha: ADD,
                    }),
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
        Stage { pipeline, write }
    }
}

/// Which of the two scene colours holds the picture so far. The world is drawn
/// into the first; each stage reads one and writes the other.
#[derive(Default)]
pub struct Cursor(usize);

pub struct Composer {
    input_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    cloud_march: Stage,
    cloud_lay: Stage,
    bloom_bright: Stage,
    bloom_down: Stage,
    bloom_up: Stage,
    bloom_lay: Stage,
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
        let bloom = |entry, write| {
            Stage::new(
                device,
                "bloom",
                include_str!("shaders/bloom.wgsl"),
                entry,
                &layouts,
                SCENE_FORMAT,
                write,
            )
        };
        Composer {
            cloud_march: Stage::new(
                device,
                "cloud march",
                include_str!("shaders/clouds.wgsl"),
                "fs_march",
                &layouts,
                SCENE_FORMAT,
                Write::Replace,
            ),
            cloud_lay: Stage::new(
                device,
                "cloud lay",
                include_str!("shaders/clouds.wgsl"),
                "fs_lay",
                &layouts,
                SCENE_FORMAT,
                Write::Replace,
            ),
            bloom_bright: bloom("fs_bright", Write::Replace),
            bloom_down: bloom("fs_down", Write::Replace),
            bloom_up: bloom("fs_up", Write::Add),
            bloom_lay: bloom("fs_lay", Write::Replace),
            output: Stage::new(
                device,
                "output",
                include_str!("shaders/output.wgsl"),
                "fs",
                &layouts,
                format,
                Write::Replace,
            ),
            input_layout,
            sampler,
        }
    }

    /// The clouds over what is drawn so far. Before the sea for a camera under
    /// it, which sees them through its surface; after it for any other.
    pub fn clouds(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::BindGroup,
        targets: &Targets,
        at: &mut Cursor,
    ) {
        // Marched at half size from the depth alone, laid at full size.
        Self::pass(
            encoder,
            &self.cloud_march,
            view,
            &targets.marching[at.0],
            &targets.half,
        );
        let write = 1 - at.0;
        Self::pass(
            encoder,
            &self.cloud_lay,
            view,
            &targets.inputs[at.0],
            &targets.color[write],
        );
        at.0 = write;
    }

    /// Copies the picture so far aside and returns where the sea is to be
    /// drawn, over the original, and what it reads: the copy and the depth.
    pub fn behind<'a>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        targets: &'a Targets,
        at: &Cursor,
    ) -> (&'a wgpu::TextureView, &'a wgpu::BindGroup) {
        let whole = |texture| wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        encoder.copy_texture_to_texture(
            whole(&targets.textures[at.0]),
            whole(&targets.textures[1 - at.0]),
            wgpu::Extent3d {
                width: targets.size[0],
                height: targets.size[1],
                depth_or_array_layers: 1,
            },
        );
        (&targets.color[at.0], &targets.inputs[1 - at.0])
    }

    /// The rest of the chain, and the picture into `target`.
    pub fn finish(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::BindGroup,
        targets: &Targets,
        mut at: Cursor,
        bloom: bool,
        target: &wgpu::TextureView,
    ) {
        if bloom {
            // What is bright, halved down a pyramid, then summed back up it:
            // each level adds a wider, fainter glow. Laid over the scene last.
            let glow = &targets.glow;
            Self::pass(
                encoder,
                &self.bloom_bright,
                view,
                &targets.inputs[at.0],
                &glow[0].0,
            );
            for level in 1..BLOOM_LEVELS {
                Self::pass(
                    encoder,
                    &self.bloom_down,
                    view,
                    &glow[level - 1].1,
                    &glow[level].0,
                );
            }
            for level in (1..BLOOM_LEVELS).rev() {
                Self::pass(
                    encoder,
                    &self.bloom_up,
                    view,
                    &glow[level].1,
                    &glow[level - 1].0,
                );
            }
            let write = 1 - at.0;
            Self::pass(
                encoder,
                &self.bloom_lay,
                view,
                &targets.glowing[at.0],
                &targets.color[write],
            );
            at.0 = write;
        }
        Self::pass(encoder, &self.output, view, &targets.inputs[at.0], target);
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
                    load: match stage.write {
                        // Every pixel is written: what was there is no use.
                        Write::Replace => wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        Write::Add => wgpu::LoadOp::Load,
                    },
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

    /// The layout of what the sea reads: the scene behind it and its depth.
    pub fn behind_layout(&self) -> &wgpu::BindGroupLayout {
        &self.input_layout
    }

    /// The targets of one view, at its size.
    pub fn targets(&self, device: &wgpu::Device, size: [u32; 2]) -> Targets {
        let raw = |label, format, usage, divide: u32| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size[0].div_ceil(divide).max(1),
                    height: size[1].div_ceil(divide).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let texture = |label, format, usage, divide: u32| {
            raw(label, format, usage, divide).create_view(&wgpu::TextureViewDescriptor::default())
        };
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        // Either is copied to the other for the sea to read.
        let copied = usage | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        let textures = [
            raw("scene colour", SCENE_FORMAT, copied, 1),
            raw("stage colour", SCENE_FORMAT, copied, 1),
        ];
        let color =
            [0, 1].map(|i| textures[i].create_view(&wgpu::TextureViewDescriptor::default()));
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
        // Level n is the scene halved n + 1 times. Each with itself as input.
        let glow: [(wgpu::TextureView, wgpu::BindGroup); BLOOM_LEVELS] =
            core::array::from_fn(|level| {
                let view = texture("bloom level", SCENE_FORMAT, usage, 2 << level);
                let group = input(&view, &half);
                (view, group)
            });
        // The scene with the summed glow beside it, for the stage that lays it.
        let glowing = [input(&color[0], &glow[0].0), input(&color[1], &glow[0].0)];
        // The half size target cannot be read while it is written.
        let marching = [input(&color[0], &color[1]), input(&color[1], &color[0])];
        Targets {
            size,
            textures,
            color,
            half,
            depth,
            inputs,
            marching,
            glow,
            glowing,
        }
    }
}

/// Where one view's world is drawn and its stages run.
pub struct Targets {
    pub size: [u32; 2],
    textures: [wgpu::Texture; 2],
    /// 0 takes the scene; the stages alternate between the two.
    pub color: [wgpu::TextureView; 2],
    pub depth: wgpu::TextureView,
    /// Where a costly stage works, half the size each way.
    half: wgpu::TextureView,
    inputs: [wgpu::BindGroup; 2],
    /// The picture's inputs for the stage that writes `half`.
    marching: [wgpu::BindGroup; 2],
    glow: [(wgpu::TextureView, wgpu::BindGroup); BLOOM_LEVELS],
    glowing: [wgpu::BindGroup; 2],
}
