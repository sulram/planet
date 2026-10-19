// Cubes of the build layer, placed by the offset of their mesh: every corner
// is bent onto the body already. Their sides are cubes.wgsl's.

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
