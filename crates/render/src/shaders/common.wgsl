// Shared by every pipeline: the per view uniforms, the atmosphere, the output
// encoding. Positions named `relative` are metres from the camera; nothing on
// the GPU is ever in planet space except the camera itself.

struct View {
    clip_from_relative: mat4x4<f32>,
    relative_from_clip: mat4x4<f32>,
    // xyz: camera from the planet centre, w: planet (sea level) radius.
    camera: vec4<f32>,
    // xyz: unit vector to the sun, w: radius of the top of the atmosphere.
    sun: vec4<f32>,
    // x: 1 when the target is not sRGB and the shader must encode.
    flags: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

const SKY: vec3<f32> = vec3<f32>(0.30, 0.55, 1.00);
const SUNSET: vec3<f32> = vec3<f32>(1.00, 0.45, 0.18);
const DENSITY_PER_M: f32 = 0.0001;

// Distances along the ray to a sphere at the planet centre. x > y is a miss.
fn ray_sphere(origin: vec3<f32>, dir: vec3<f32>, radius: f32) -> vec2<f32> {
    let b = dot(origin, dir);
    let c = dot(origin, origin) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return vec2<f32>(1.0, -1.0);
    }
    let root = sqrt(disc);
    return vec2<f32>(-b - root, -b + root);
}

// Light scattered toward the camera along a ray of `length` metres (rgb), and
// how much of what lies behind survives (a).
fn atmosphere(dir: vec3<f32>, length: f32) -> vec4<f32> {
    let origin = view.camera.xyz;
    let shell = ray_sphere(origin, dir, view.sun.w);
    let enter = max(shell.x, 0.0);
    let leave = min(shell.y, length);
    let path = leave - enter;
    if path <= 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let through = exp(-path * DENSITY_PER_M);

    // Daylight where the ray runs, warm when the sun is low.
    let middle = normalize(origin + dir * (enter + path * 0.5));
    let sun_height = dot(middle, view.sun.xyz);
    let day = smoothstep(-0.20, 0.25, sun_height);
    let warm = (1.0 - smoothstep(0.0, 0.40, sun_height)) * max(dot(dir, view.sun.xyz), 0.0);
    let tint = mix(SKY, SUNSET, warm);
    // Long paths wash out toward white, as the real horizon does.
    let haze = mix(tint, vec3<f32>(1.0), (1.0 - through) * 0.45);
    return vec4<f32>(haze * (1.0 - through) * day * 1.5, through);
}

fn encode(linear: vec3<f32>) -> vec4<f32> {
    let c = clamp(linear, vec3<f32>(0.0), vec3<f32>(1.0));
    if view.flags.x < 0.5 {
        return vec4<f32>(c, 1.0);
    }
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return vec4<f32>(select(high, low, c <= vec3<f32>(0.0031308)), 1.0);
}

// Sun and sky light on a surface, then the air between it and the camera.
fn lit(albedo: vec3<f32>, normal: vec3<f32>, gloss: f32, relative: vec3<f32>) -> vec3<f32> {
    let distance = length(relative);
    let dir = relative / distance;
    let up = normalize(view.camera.xyz + relative);
    let sun = view.sun.xyz;

    // The planet shadows itself: daylight fades as the sun sets on this spot.
    let day = smoothstep(-0.10, 0.15, dot(up, sun));
    let direct = max(dot(normal, sun), 0.0) * day;
    let ambient = 0.05 + 0.25 * day * (0.5 + 0.5 * dot(normal, up));
    var color = albedo * (direct * vec3<f32>(1.0, 0.96, 0.90) + ambient * SKY);

    let half_vector = normalize(sun - dir);
    color += gloss * day * pow(max(dot(normal, half_vector), 0.0), 120.0) * 0.8;

    let air = atmosphere(dir, distance);
    return color * air.a + air.rgb;
}
