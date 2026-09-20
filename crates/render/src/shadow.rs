//! Sun shadow cascades around the eye, stable in the reference frame of either
//! body. Matrices are built in f64 and act on camera-relative f32 positions.

use glam::{DMat4, DVec3, Mat4, Vec3};
use scene::{Camera, Frame};

pub const CASCADES: usize = 3;
pub const SIZE: u32 = 1024;
/// Half side of each cascade, metres: contact shadows of an avatar, the relief
/// walked through, and the mountains that shade the horizon at a low sun.
pub const RADII: [f64; CASCADES] = [40.0, 400.0, 4000.0];
/// How far towards the sun, and away from it, a cascade holds casters, metres.
/// A low sun throws a mountain's shadow several times its height.
const REACH: [f64; CASCADES] = [2400.0, 2400.0, 12000.0];

/// What the shaders must agree on, stated once: prepended to every module.
pub fn prelude() -> String {
    format!("const SHADOW_CASCADES: u32 = {CASCADES}u;\nconst SHADOW_SIZE: f32 = {SIZE}.0;\n")
}

/// Side of one texel of each cascade on the ground, metres.
pub fn texels_m() -> [f32; 4] {
    let mut texels = [0.0; 4];
    for (texel, radius) in texels.iter_mut().zip(RADII) {
        *texel = (2.0 * radius / f64::from(SIZE)) as f32;
    }
    texels
}

pub fn matrices(frame: &Frame, camera: &Camera) -> [Mat4; CASCADES] {
    let moon = frame.moon.position;
    let on_moon = (camera.position - moon).length() - frame.moon.radius_m
        < camera.position.length() - frame.planet_radius_m;
    let origin = if on_moon { moon } else { DVec3::ZERO };
    let sun = frame.sun_direction.as_dvec3().normalize();
    let helper = if sun.y.abs() < 0.95 {
        DVec3::Y
    } else {
        DVec3::X
    };
    let rotation = DMat4::look_to_rh(DVec3::ZERO, -sun, helper);
    let position = rotation.transform_point3(camera.position - origin);
    core::array::from_fn(|cascade| {
        let (radius, reach) = (RADII[cascade], REACH[cascade]);
        let texel = 2.0 * radius / f64::from(SIZE);
        let snap = DVec3::new(
            (position.x / texel).round() * texel,
            (position.y / texel).round() * texel,
            position.z,
        );
        // Light looks down -Z; reverse near/far to retain reversed-Z everywhere.
        let projection = DMat4::orthographic_rh(-radius, radius, -radius, radius, 2.0 * reach, 0.0);
        let offset = position - snap - DVec3::Z * reach;
        (projection * DMat4::from_translation(offset) * rotation).as_mat4()
    })
}

/// Conservative sphere test, including casters outside the camera frustum.
pub fn intersects(matrix: Mat4, center: Vec3, radius: f32) -> bool {
    let p = matrix.transform_point3(center);
    let rows = matrix.transpose();
    let r = Vec3::new(
        rows.x_axis.truncate().length(),
        rows.y_axis.truncate().length(),
        rows.z_axis.truncate().length(),
    ) * radius;
    p.x.abs() <= 1.0 + r.x && p.y.abs() <= 1.0 + r.y && p.z + r.z >= 0.0 && p.z - r.z <= 1.0
}

pub struct Maps {
    pub sampled: wgpu::TextureView,
    pub layers: [wgpu::TextureView; CASCADES],
    pub sampler: wgpu::Sampler,
    pub uniforms: [wgpu::Buffer; CASCADES],
    pub groups: [wgpu::BindGroup; CASCADES],
}

impl Maps {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sun shadow maps"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: CASCADES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: super::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layers = std::array::from_fn(|i| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: i as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sun PCF"),
            compare: Some(wgpu::CompareFunction::GreaterEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniforms = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sun view"),
                size: size_of::<super::ViewUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let groups = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sun view"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms[i].as_entire_binding(),
                }],
            })
        });
        Self {
            sampled,
            layers,
            sampler,
            uniforms,
            groups,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn caster_bounds_include_offscreen_and_reject_outside_light_volume() {
        let m = Mat4::orthographic_rh(-32.0, 32.0, -32.0, 32.0, 4800.0, 0.0);
        assert!(intersects(m, Vec3::new(0.0, 0.0, -1000.0), 1.0));
        assert!(intersects(m, Vec3::new(33.0, 0.0, -1000.0), 2.0));
        assert!(!intersects(m, Vec3::new(40.0, 0.0, -1000.0), 1.0));
        assert!(!intersects(m, Vec3::new(0.0, 0.0, 10.0), 1.0));
    }
}
