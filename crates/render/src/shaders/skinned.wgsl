const MAX_JOINTS: u32 = 128u;

struct Instance {
    relative_from_mesh: mat4x4<f32>,
    // Mesh space from rest space, per joint.
    joints: array<mat4x4<f32>, MAX_JOINTS>,
}

@group(1) @binding(0) var<uniform> instance: Instance;
@group(2) @binding(0) var base_color: texture_2d<f32>;
@group(2) @binding(1) var base_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) joints: vec4<u32>,
    @location(4) weights: vec4<f32>,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs(in: Vertex) -> Varying {
    let skin = instance.joints[in.joints.x] * in.weights.x
        + instance.joints[in.joints.y] * in.weights.y
        + instance.joints[in.joints.z] * in.weights.z
        + instance.joints[in.joints.w] * in.weights.w;
    let placed = instance.relative_from_mesh * skin;
    var out: Varying;
    out.relative = (placed * vec4<f32>(in.position, 1.0)).xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    // Joints and placement are rigid (no shear), so the matrix itself carries
    // normals; normalising undoes any uniform scale.
    out.normal = (placed * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv = in.uv;
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    let texel = textureSample(base_color, base_sampler, in.uv);
    if texel.a < 0.5 {
        discard;
    }
    return encode(lit(texel.rgb, normalize(in.normal), 0.0, in.relative));
}
