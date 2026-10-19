// One level of a mip chain from the one above it: a screen triangle over the
// level being written, reading the finer level bilinearly at its centre.

@group(0) @binding(0) var finer: texture_2d<f32>;
@group(0) @binding(1) var finer_sampler: sampler;

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> Varying {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: Varying;
    out.clip = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    return textureSample(finer, finer_sampler, in.uv);
}
