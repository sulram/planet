struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

// One box: `relative_from_box` as columns, then linear colour.
struct Instance {
    @location(2) c0: vec4<f32>,
    @location(3) c1: vec4<f32>,
    @location(4) c2: vec4<f32>,
    @location(5) c3: vec4<f32>,
    @location(6) color: vec4<f32>,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

@vertex
fn vs(in: Vertex, box: Instance) -> Varying {
    let relative_from_box = mat4x4<f32>(box.c0, box.c1, box.c2, box.c3);
    var out: Varying;
    out.relative = (relative_from_box * vec4<f32>(in.position, 1.0)).xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    // Boxes scale along their own axes only, so dividing the rotated normal
    // by the squared axis lengths is the inverse transpose.
    let m = mat3x3<f32>(box.c0.xyz, box.c1.xyz, box.c2.xyz);
    let scale2 = vec3<f32>(dot(m[0], m[0]), dot(m[1], m[1]), dot(m[2], m[2]));
    out.normal = m * (in.normal / scale2);
    out.color = box.color.rgb;
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    return vec4<f32>(lit(in.color, normalize(in.normal), 0.0, in.relative), 1.0);
}
