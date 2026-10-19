// A picture: a quad with a texture, lit as a matte surface or shining with
// its own colour, as a cell's side does.

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// One picture: `relative_from_quad` as columns, then how much it shines.
struct Instance {
    @location(3) c0: vec4<f32>,
    @location(4) c1: vec4<f32>,
    @location(5) c2: vec4<f32>,
    @location(6) c3: vec4<f32>,
    @location(7) params: vec4<f32>,
}

@group(1) @binding(0) var picture: texture_2d<f32>;
@group(1) @binding(1) var picture_sampler: sampler;

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) shine: f32,
}

@vertex
fn vs(in: Vertex, quad: Instance) -> Varying {
    let relative_from_quad = mat4x4<f32>(quad.c0, quad.c1, quad.c2, quad.c3);
    var out: Varying;
    out.relative = (relative_from_quad * vec4<f32>(in.position, 1.0)).xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    // A quad scales along its own axes only, so dividing the rotated normal
    // by the squared axis lengths is the inverse transpose.
    let m = mat3x3<f32>(quad.c0.xyz, quad.c1.xyz, quad.c2.xyz);
    let scale2 = vec3<f32>(dot(m[0], m[0]), dot(m[1], m[1]), dot(m[2], m[2]));
    out.normal = m * (in.normal / scale2);
    out.uv = in.uv;
    out.shine = quad.params.x;
    return out;
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    let texel = textureSample(picture, picture_sampler, in.uv);
    if texel.a < 0.5 {
        discard;
    }
    let normal = normalize(in.normal);
    let lit = lit(texel.rgb, normal, 0.0, in.relative);
    // A screen shows, a lamp glows: its white is a sunlit white, no brighter.
    return vec4<f32>(mix(lit, own_light(texel.rgb, in.relative), in.shine), 1.0);
}

// The same cutout as the visible quad: what is see-through casts nothing.
@fragment
fn fs_shadow(in: Varying) {
    if textureSample(picture, picture_sampler, in.uv).a < 0.5 {
        discard;
    }
}
