// What every compositor stage starts with: the colour and depth before it, and
// one triangle over the screen.
@group(1) @binding(0) var stage_color: texture_2d<f32>;
@group(1) @binding(1) var stage_depth: texture_depth_2d;
@group(1) @binding(2) var stage_sampler: sampler;

struct Screen {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> Screen {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: Screen;
    out.clip = vec4<f32>(corner, 0.0, 1.0);
    out.ndc = corner;
    return out;
}

// The pixel this fragment stands on, in the targets before it.
fn stage_pixel(in: Screen) -> vec2<i32> {
    return vec2<i32>(in.clip.xy);
}
