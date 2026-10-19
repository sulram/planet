// A model of cubes placed by a part: its mesh is in its own space and one
// matrix puts it where the part stands, relative to the camera. Its sides
// are cubes.wgsl's.

struct Instance {
    relative_from_model: mat4x4<f32>,
    // Carries normals: the inverse transpose, so a stretched model is lit
    // as its sides face.
    normal_from_model: mat4x4<f32>,
}

@group(1) @binding(0) var<uniform> instance: Instance;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) side: vec4<f32>,
}

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = (instance.relative_from_model * vec4<f32>(in.position, 1.0)).xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.normal = normalize((instance.normal_from_model * vec4<f32>(in.normal, 0.0)).xyz);
    out.color = in.color;
    out.side = in.side;
    return out;
}
