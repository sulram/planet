// Cubes of the build layer: flat sides, lit by the same sun as the ground,
// with the sky shut out of their corners.

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
    @location(3) open: f32,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) open: f32,
}

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = in.position + placement.offset.xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    out.open = in.open;
    return out;
}

fn shade(in: Varying) -> vec3<f32> {
    let albedo = pow(in.color.rgb, vec3<f32>(2.2));
    let normal = normalize(in.normal);
    // A shut corner darkens along a curve, the way light falls off into one.
    let open = in.open * in.open;
    return lit_occluded(albedo, normal, in.color.a, in.relative, normal, mix(0.25, 1.0, open));
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    return vec4<f32>(shade(in), 1.0);
}

// The stroke a tool would make, over what is already there.
@fragment
fn fs_ghost(in: Varying) -> @location(0) vec4<f32> {
    return vec4<f32>(shade(in), 0.45);
}
