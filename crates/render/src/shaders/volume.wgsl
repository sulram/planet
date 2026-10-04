// Cubes of the build layer: flat sides, lit by the same sun as the ground,
// with the sky shut out of their corners. A side may carry an edge, a light
// shines with its own colour, and glass lets what stands behind it show.

struct Placement {
    // xyz: mesh origin relative to the camera.
    offset: vec4<f32>,
    // xyz: mesh origin in planet space, wrapped by the CPU. Unused here; the
    // slot is the one terrain binds.
    anchor: vec4<f32>,
}

@group(1) @binding(0) var<uniform> placement: Placement;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) side: vec4<f32>,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    // rgb: albedo. a: how much the side shines with its own colour.
    @location(2) color: vec4<f32>,
    // x: how open the corner is. yz: where on the side. w: its edge, of 255.
    @location(3) side: vec4<f32>,
}

// How far an edge reaches into a side, in sides: two sides meet in a line
// twice as wide.
const EDGE: f32 = 0.045;
// How many times over a light shines with its colour: enough to glow.
const SHINE: f32 = 3.0;

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = in.position + placement.offset.xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    out.side = in.side;
    return out;
}

// How much of a pixel the edge of a side covers: the share of the pixel that
// lies within EDGE of a border of the side, on either side of it. Far off,
// where an edge is thinner than a pixel, it melts into the side by as much
// as it covers of it, and never shimmers.
fn edge_cover(at: vec2<f32>) -> f32 {
    let pixel = max(fwidth(at), vec2<f32>(1e-5));
    let away = min(at, 1.0 - at);
    let reach = vec2<f32>(EDGE);
    let within = min(away + pixel * 0.5, reach) - max(away - pixel * 0.5, -reach);
    let cover = clamp(within / pixel, vec2<f32>(0.0), vec2<f32>(1.0));
    return 1.0 - (1.0 - cover.x) * (1.0 - cover.y);
}

// The colour of a side under its edge: black, white, or none.
fn inked(albedo: vec3<f32>, edge: f32, cover: f32) -> vec3<f32> {
    let kind = edge * 255.0;
    let ink = select(vec3<f32>(0.012), vec3<f32>(0.9), kind > 1.5);
    return mix(albedo, ink, cover * step(0.5, kind));
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    let cover = edge_cover(in.side.yz);
    let albedo = inked(pow(in.color.rgb, vec3<f32>(2.2)), in.side.w, cover);
    let normal = normalize(in.normal);
    // A shut corner darkens along a curve, the way light falls off into one.
    let open = in.side.x * in.side.x;
    let lit = lit_occluded(albedo, normal, 0.0, in.relative, normal, mix(0.25, 1.0, open));
    // A light is its own colour whatever falls on it, by night as by day.
    let distance = length(in.relative);
    let own = through_medium(albedo * SHINE, in.relative / distance, distance);
    return vec4<f32>(mix(lit, own, in.color.a), 1.0);
}

// Glass: its tint over what stands behind it, more of it the more it is seen
// edge on, and its edge drawn whole.
@fragment
fn fs_glass(in: Varying) -> @location(0) vec4<f32> {
    let cover = edge_cover(in.side.yz);
    let albedo = inked(pow(in.color.rgb, vec3<f32>(2.2)), in.side.w, cover);
    let normal = normalize(in.normal);
    let distance = length(in.relative);
    let glancing = pow(1.0 - abs(dot(normal, in.relative / distance)), 3.0);
    var color = lit_occluded(albedo, normal, 1.0, in.relative, normal, 1.0);
    // Seen edge on it mirrors the sky.
    let day = sunlight(in.relative) * daylight(surface_up(in.relative));
    color += SKY * 0.35 * glancing * day;
    let edged = cover * step(0.5, in.side.w * 255.0);
    return vec4<f32>(color, max(mix(0.24, 0.85, glancing), edged));
}
