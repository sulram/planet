// egui meshes over the finished picture. Vertices come in points, y down,
// origin top left, with sRGB premultiplied colours.

struct Screen {
    size_points: vec2<f32>,
    // 1 when the target is sRGB and the colours must be made linear for it.
    linear: f32,
    pad: f32,
}

@group(0) @binding(0) var<uniform> screen: Screen;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

fn linear_from_srgb(srgb: vec3<f32>) -> vec3<f32> {
    let low = srgb / 12.92;
    let high = pow((srgb + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, srgb < vec3<f32>(0.04045));
}

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.clip = vec4<f32>(
        2.0 * in.position.x / screen.size_points.x - 1.0,
        1.0 - 2.0 * in.position.y / screen.size_points.y,
        0.0,
        1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    // The atlas is sampled as it was written, sRGB: blend in that space, the
    // one egui was designed in, then hand the target what it expects.
    let color = in.color * textureSample(image, image_sampler, in.uv);
    if screen.linear < 0.5 {
        return color;
    }
    return vec4<f32>(linear_from_srgb(color.rgb), color.a);
}
