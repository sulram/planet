//! Rendering without a window: a texture target and a way to read it back.
//! This is how every change gets looked at (CLAUDE.md, How it grows).

use std::path::Path;

use scene::Frame;

use crate::{Gpu, Renderer, View};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// What a painter over the headless picture is handed.
pub struct Over<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub target: &'a wgpu::TextureView,
    pub size: [u32; 2],
}

/// The format of the headless picture.
pub const HEADLESS_FORMAT: wgpu::TextureFormat = FORMAT;

pub struct Headless {
    gpu: Gpu,
    pub renderer: Renderer,
    size: [u32; 2],
    texture: wgpu::Texture,
}

impl Headless {
    pub fn new(width: u32, height: u32) -> Result<Headless, String> {
        let instance = wgpu::Instance::default();
        let gpu = pollster::block_on(Gpu::new(&instance, None))?;
        let renderer = Renderer::new(&gpu, FORMAT);
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Ok(Headless {
            gpu,
            renderer,
            size: [width, height],
            texture,
        })
    }

    /// CPU submission plus GPU completion, without readback or PNG encoding.
    /// Warm up first. This is a fixed-scene diagnostic, not interactive FPS.
    pub fn measure(&mut self, frame: &Frame, count: usize) -> (f64, f64) {
        let count = count.max(1);
        let target = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut elapsed = Vec::with_capacity(count);
        for index in 0..count + 8 {
            let start = std::time::Instant::now();
            self.renderer.render(
                frame,
                &[View {
                    camera: frame.camera,
                    target: &target,
                    size: self.size,
                }],
            );
            self.gpu
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .expect("GPU completion");
            if index >= 8 {
                elapsed.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        elapsed.sort_by(f64::total_cmp);
        (
            elapsed[count / 2],
            elapsed[((count as f64 * 0.95) as usize).min(count - 1)],
        )
    }

    /// Renders the frame and returns its pixels, RGBA8 sRGB, top row first.
    pub fn render(&mut self, frame: &Frame) -> Vec<u8> {
        self.render_under(frame, |_| {})
    }

    /// The same, with something painted over the picture before it is read: a
    /// native UI is looked at the way the world is.
    pub fn render_under(&mut self, frame: &Frame, over: impl FnOnce(Over<'_>)) -> Vec<u8> {
        let [width, height] = self.size;
        let target = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.renderer.render(
            frame,
            &[View {
                camera: frame.camera,
                target: &target,
                size: self.size,
            }],
        );

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("over"),
            });
        over(Over {
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            encoder: &mut encoder,
            target: &target,
            size: self.size,
        });
        self.gpu.queue.submit([encoder.finish()]);

        // Rows in a readback buffer are padded to the copy alignment.
        let stride = (4 * width).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.gpu.queue.submit([encoder.finish()]);

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {
            result.expect("failed to map the readback buffer");
        });
        self.gpu
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("device lost while reading a frame back");
        let mapped = slice
            .get_mapped_range()
            .expect("readback buffer is not mapped");
        mapped
            .chunks_exact(stride as usize)
            .flat_map(|row| &row[..4 * width as usize])
            .copied()
            .collect()
    }
}

pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())
}
