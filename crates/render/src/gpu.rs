//! Bringing up a device, the same way on every platform.

/// An adapter's device and queue. Cheap to clone pieces out of.
pub struct Gpu {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// With a `surface` the adapter is one that can present to it; without,
    /// any adapter will do (headless).
    pub async fn new(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Gpu, String> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| format!("no compatible GPU adapter: {error}"))?;
        log::info!("adapter: {:?}", adapter.get_info());

        // The renderer needs nothing beyond what WebGL2 offers, so the floor
        // is the downlevel defaults, raised to what this adapter has.
        let required_limits =
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("planet"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| format!("failed to open the GPU device: {error}"))?;
        Ok(Gpu {
            adapter,
            device,
            queue,
        })
    }
}

/// The configuration both windowed shells present with. The format is sRGB
/// when the surface offers one; when it does not (browsers), the renderer
/// encodes in the shader instead.
pub fn surface_configuration(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> wgpu::SurfaceConfiguration {
    let formats = surface.get_capabilities(adapter).formats;
    let srgb = formats.iter().copied().find(wgpu::TextureFormat::is_srgb);
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: srgb.unwrap_or(formats[0]),
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        color_space: wgpu::SurfaceColorSpace::default(),
    }
}
