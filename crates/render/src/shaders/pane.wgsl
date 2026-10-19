// A portal's pane: the disc in a quad. Seen through, it is a window: the
// world past the portal, drawn from the eye carried through it with this
// view's own projection, sampled where the pane stands on the screen, bent
// toward the rim in a band that pulses, the colours parting there. Not
// seen through, it is a lens: what is behind it, bent toward its middle
// and pulsing, as through thick glass, with a faint rim. Between, the one
// becomes the other.

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// One pane: `relative_from_quad` as columns, then how much is seen through.
struct Instance {
    @location(3) c0: vec4<f32>,
    @location(4) c1: vec4<f32>,
    @location(5) c2: vec4<f32>,
    @location(6) c3: vec4<f32>,
    @location(7) params: vec4<f32>,
}

@group(1) @binding(0) var through: texture_2d<f32>;
@group(1) @binding(1) var through_sampler: sampler;
// The picture so far, copied: what is behind the pane.
@group(2) @binding(0) var behind: texture_2d<f32>;
@group(2) @binding(2) var behind_sampler: sampler;

// How much a lens bends what is behind it, of the screen, and how much
// that swells and ebbs, and how fast, a second.
const BEND: f32 = 0.06;
const PULSE: f32 = 0.35;
const PACE: f32 = 2.2;
// The lens's tint, and its rim's light.
const TINT: vec3<f32> = vec3<f32>(0.86, 0.9, 1.0);
const RIM: vec3<f32> = vec3<f32>(0.9, 0.85, 1.2);

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) seen: f32,
    // The clip position again, interpolated: where this point is on the
    // screen, which is where the world past the portal was drawn for it,
    // and where what is behind it is.
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

// How much the window bends what it shows toward its rim, of the screen's
// height, how far in from the rim the bend reaches, of the radius, and how
// much the colours part there.
const WINDOW_BEND: f32 = 0.035;
const WINDOW_FALL: f32 = 0.35;
const WINDOW_SPLIT: f32 = 0.5;

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    let ndc = in.screen.xy / in.screen.w;
    let at = vec2<f32>(ndc.x, -ndc.y) * 0.5 + 0.5;
    // Where on the disc, from its middle: 1 at its rim.
    let off = (in.uv - 0.5) * 2.0;
    let r = length(off);
    let pulse = 1.0 + PULSE * sin(view.flags.y * PACE);
    // The lens: what is behind, read nearer the middle than it is, more so
    // toward the rim, the bend swelling and ebbing.
    let bend = BEND * pulse * r * r * (1.0 - r * r);
    let bent = clamp(at - off * bend, vec2<f32>(0.0), vec2<f32>(1.0));
    let glass = textureSample(behind, behind_sampler, bent).rgb * TINT;
    let rim = pow(r, 8.0) * (0.4 + 0.2 * sin(view.flags.y * PACE + 1.0));
    // The window: the world past the portal, bent toward the rim along the
    // rim's own direction on the screen, the colours parting a little
    // there, so the picture, drawn smaller than the view, meets the view
    // through a band that is the portal's and not a seam.
    let toward = vec2<f32>(dpdx(r), dpdy(r));
    let normal = toward / max(length(toward), 1e-6);
    let edge = smoothstep(1.0 - WINDOW_FALL, 1.0, r) * (0.7 + 0.3 * sin(view.flags.y * PACE * 1.3 + r * 12.0));
    let size = vec2<f32>(textureDimensions(through));
    let flat = vec2<f32>(size.y / size.x, 1.0);
    let pull = normal * flat * WINDOW_BEND * edge;
    let split = pull * WINDOW_SPLIT;
    let past = vec3<f32>(
        textureSample(through, through_sampler, clamp(at - pull - split, vec2<f32>(0.0), vec2<f32>(1.0))).r,
        textureSample(through, through_sampler, clamp(at - pull, vec2<f32>(0.0), vec2<f32>(1.0))).g,
        textureSample(through, through_sampler, clamp(at - pull + split, vec2<f32>(0.0), vec2<f32>(1.0))).b,
    );
    if r > 1.0 {
        discard;
    }
    let lens = glass + RIM * rim;
    let window = past + RIM * rim * 0.5;
    return vec4<f32>(mix(lens, window, clamp(in.seen, 0.0, 1.0)), 1.0);
}
