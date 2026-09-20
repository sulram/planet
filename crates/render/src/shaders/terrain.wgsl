struct Patch {
    // xyz: patch origin relative to the camera.
    offset: vec4<f32>,
}

@group(1) @binding(0) var<uniform> placement: Patch;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = in.position + placement.offset.xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    // Vertex colours are sRGB; alpha carries gloss (water).
    let albedo = pow(in.color.rgb, vec3<f32>(2.2));
    return encode(lit(albedo, normalize(in.normal), in.color.a, in.relative));
}
