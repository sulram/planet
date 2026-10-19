// A portal's pane: the disc in a quad. Seen through, it is a window: the
// world past the portal, drawn from the eye carried through it with this
// view's own projection, sampled where the pane stands on the screen. Else
// a dark pane, lit with its own faint light.

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// One pane: `relative_from_quad` as columns, then whether it is seen through.
struct Instance {
    @location(3) c0: vec4<f32>,
    @location(4) c1: vec4<f32>,
    @location(5) c2: vec4<f32>,
    @location(6) c3: vec4<f32>,
    @location(7) params: vec4<f32>,
}

@group(1) @binding(0) var through: texture_2d<f32>;
@group(1) @binding(1) var through_sampler: sampler;

// What a pane is while nothing is seen through it, linear light: near
// black, a little violet.
const PANE: vec3<f32> = vec3<f32>(0.0037, 0.0024, 0.0106);

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) seen: f32,
    // The clip position again, interpolated: where this point is on the
    // screen, which is where the world past the portal was drawn for it.
    @location(3) screen: vec4<f32>,
}

@vertex
fn vs(in: Vertex, pane: Instance) -> Varying {
    let relative_from_quad = mat4x4<f32>(pane.c0, pane.c1, pane.c2, pane.c3);
    var out: Varying;
    out.relative = (relative_from_quad * vec4<f32>(in.position, 1.0)).xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.screen = out.clip;
    out.uv = in.uv;
    out.seen = pane.params.x;
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    let ndc = in.screen.xy / in.screen.w;
    let at = vec2<f32>(ndc.x, -ndc.y) * 0.5 + 0.5;
    // Sampled before anything is discarded, in uniform control flow.
    let past = textureSample(through, through_sampler, at).rgb;
    if length(in.uv - 0.5) > 0.5 {
        discard;
    }
    let dark = own_light(PANE, in.relative);
    return vec4<f32>(mix(dark, past, step(0.5, in.seen)), 1.0);
}
