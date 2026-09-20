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
    // xyz: centre of the moon relative to the camera, w: its radius, metres.
    moon: vec4<f32>,
    // xyz: unit vector from the planet to the moon, for moonlight.
    moon_light: vec4<f32>,
    // x: 1 when the target is not sRGB and the shader must encode.
    // y: world clock, seconds, wrapped. z: camera height over the sea, metres.
    flags: vec4<f32>,
    shadow_clip: array<mat4x4<f32>, SHADOW_CASCADES>,
    // Side of a texel of each cascade on the ground, metres.
    shadow_texel_m: vec4<f32>,
    interaction_start: vec4<f32>,
    interaction_end: vec4<f32>,
    // x: exposure.
    post: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var shadow_map: texture_depth_2d_array;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;

fn shadow_sample(relative: vec3<f32>, normal: vec3<f32>, cascade: u32) -> vec2<f32> {
    let texel_m = view.shadow_texel_m[cascade];
    let facing = clamp(dot(normal, view.sun.xyz), 0.0, 1.0);
    let offset = normal * texel_m * (0.3 + 1.0 - facing);
    let p = (view.shadow_clip[cascade] * vec4<f32>(relative + offset, 1.0)).xyz;
    let coverage = 1.0 - smoothstep(0.78, 0.97, max(abs(p.x), abs(p.y)));
    if coverage <= 0.0 || p.z <= 0.0 || p.z >= 1.0 {
        return vec2<f32>(1.0, 0.0);
    }
    let uv = p.xy * vec2<f32>(0.5, -0.5) + 0.5;
    var sum = 0.0;
    // Four bilinear comparison samples form a small, deterministic PCF kernel.
    for (var y = 0; y < 2; y++) {
        for (var x = 0; x < 2; x++) {
            let tap = (vec2<f32>(f32(x), f32(y)) - 0.5) / SHADOW_SIZE;
            sum += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + tap, i32(cascade), p.z + 0.000008);
        }
    }
    let valid = select(0.0, coverage, p.z > 0.0 && p.z < 1.0);
    return vec2<f32>(sum * 0.25, valid);
}

fn terrain_shadow(relative: vec3<f32>, normal: vec3<f32>) -> f32 {
    if view.flags.w < 0.5 { return 1.0; }
    // Finest cascade first; each hands what its rim does not cover to the next.
    var lit = 0.0;
    var left = 1.0;
    for (var cascade = 0u; cascade < SHADOW_CASCADES; cascade++) {
        let sample = shadow_sample(relative, normal, cascade);
        lit += left * sample.y * sample.x;
        left *= 1.0 - sample.y;
        if left <= 0.0 { break; }
    }
    return lit + left;
}

const SKY: vec3<f32> = vec3<f32>(0.30, 0.55, 1.00);
const SUNSET: vec3<f32> = vec3<f32>(1.00, 0.45, 0.18);
const DENSITY_PER_M: f32 = 0.00004;

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
    let haze = mix(tint, vec3<f32>(1.0), (1.0 - through) * 0.30);
    return vec4<f32>(haze * (1.0 - through) * day * 1.1, through);
}

// Water takes red first, then green; blue travels furthest. Per metre.
const WATER_ABSORB: vec3<f32> = vec3<f32>(0.35, 0.070, 0.045);
// What the water itself scatters back: the colour of "nothing but sea".
const WATER_SCATTER: vec3<f32> = vec3<f32>(0.012, 0.115, 0.170);

// How much of the sun reaches a point (relative to the camera) past a sphere:
// 0 in its shadow, 1 clear of it, soft across the edge. Only a sphere between
// the point and the sun can shadow it. On the sphere's own surface that is the
// night side; the day side is left to the surface normal, as for any light.
fn past_sphere(relative: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
    let to_center = center - relative;
    let along = dot(to_center, view.sun.xyz);
    let miss = sqrt(max(dot(to_center, to_center) - along * along, 0.0));
    let soft = radius * 0.06;
    let shadow = smoothstep(radius - soft, radius + soft, miss);
    // Ease in across the terminator, where the sphere is just coming between.
    return mix(1.0, shadow, smoothstep(0.0, radius * 0.15, along));
}

// Sunlight at a point: whatever neither body shadows.
fn sunlight(relative: vec3<f32>) -> f32 {
    let planet = past_sphere(relative, -view.camera.xyz, view.camera.w);
    let moon = past_sphere(relative, view.moon.xyz, view.moon.w);
    return planet * moon;
}

// Up at a point: away from the body whose surface is nearer.
fn surface_up(relative: vec3<f32>) -> vec3<f32> {
    let from_planet = view.camera.xyz + relative;
    let from_moon = relative - view.moon.xyz;
    let nearer_moon = length(from_moon) - view.moon.w < length(from_planet) - view.camera.w;
    return normalize(select(from_planet, from_moon, nearer_moon));
}

// Daylight at a place on the planet, 0 at night.
fn daylight(up: vec3<f32>) -> f32 {
    return smoothstep(-0.10, 0.15, dot(up, view.sun.xyz));
}

// What the medium between the camera and a surface does to its colour: air
// above the sea, water below it.
fn through_medium(color: vec3<f32>, dir: vec3<f32>, distance: f32) -> vec3<f32> {
    if view.flags.z < 0.0 {
        let day = daylight(normalize(view.camera.xyz));
        // Each colour dies at its own rate along the way, and the sea's own
        // glow fills in. Both dim with the depth of the camera.
        let survive = exp(-WATER_ABSORB * distance);
        let glow = WATER_SCATTER * (0.08 + day) * exp(view.flags.z * 0.025);
        return color * survive + glow * (1.0 - survive);
    }
    let air = atmosphere(dir, distance);
    return color * air.a + air.rgb;
}

// Sun and sky light on a surface, then the medium between it and the camera.
fn lit(albedo: vec3<f32>, normal: vec3<f32>, gloss: f32, relative: vec3<f32>) -> vec3<f32> {
    return lit_surface(albedo,normal,gloss,relative,normal);
}

fn lit_surface(albedo: vec3<f32>, normal: vec3<f32>, gloss: f32, relative: vec3<f32>, geometric_normal: vec3<f32>) -> vec3<f32> {
    let distance = length(relative);
    let dir = relative / distance;
    let up = surface_up(relative);
    let sun = view.sun.xyz;

    // Bodies shadow themselves and each other: night, and eclipses.
    let day = sunlight(relative);
    let visibility = terrain_shadow(relative, geometric_normal);
    let direct = max(dot(normal, sun), 0.0) * day * visibility;
    // Sky from above, warm bounce from the ground below: shadowed sides keep
    // their own colour instead of going blue.
    let facing_sky = 0.5 + 0.5 * dot(normal, up);
    // A cast shadow also hides the bright sky around the sun and the ground
    // it would have lit: less fill in there, or relief reads as a tint.
    let fill = mix(0.45, 1.0, visibility);
    let ambient = mix(vec3<f32>(0.10, 0.09, 0.07), SKY * 0.38 + 0.06, facing_sky) * day * fill
        // Starlight: nights are dark, never blind.
        + vec3<f32>(0.045, 0.058, 0.095) * facing_sky;
    var color = albedo * (direct * vec3<f32>(1.75, 1.66, 1.5) + ambient);

    // Moonlight: faint, and only as much as the moon is lit and up.
    let moon = view.moon_light.xyz;
    let moon_up = smoothstep(-0.05, 0.15, dot(up, moon));
    let moon_lit = 0.5 - 0.5 * dot(moon, sun);
    color += albedo * max(dot(normal, moon), 0.0) * moon_up * moon_lit * vec3<f32>(0.10, 0.12, 0.16);

    let half_vector = normalize(sun - dir);
    color += gloss * day * visibility * pow(max(dot(normal, half_vector), 0.0), 90.0) * 1.5;
    return through_medium(color, dir, distance);
}

// Value noise on a lattice that repeats every `period` cells, so it can be
// fed positions wrapped by the CPU in f64 (see `placement.anchor`).
fn lattice(cell: vec3<i32>, period: i32) -> f32 {
    let c = vec3<u32>((cell % period + period) % period);
    var h = (c.x * 0x8da6b343u) ^ (c.y * 0xd8163841u) ^ (c.z * 0xcb1ab31fu);
    h = (h ^ (h >> 15u)) * 0x2c1b3c6du;
    h = (h ^ (h >> 12u)) * 0x297a2d39u;
    return f32((h ^ (h >> 15u)) & 0xffffu) / 65535.0;
}

fn value_noise(p: vec3<f32>, period: i32) -> f32 {
    let cell = vec3<i32>(floor(p));
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);
    let x00 = mix(lattice(cell, period), lattice(cell + vec3<i32>(1, 0, 0), period), w.x);
    let x10 = mix(lattice(cell + vec3<i32>(0, 1, 0), period), lattice(cell + vec3<i32>(1, 1, 0), period), w.x);
    let x01 = mix(lattice(cell + vec3<i32>(0, 0, 1), period), lattice(cell + vec3<i32>(1, 0, 1), period), w.x);
    let x11 = mix(lattice(cell + vec3<i32>(0, 1, 1), period), lattice(cell + vec3<i32>(1, 1, 1), period), w.x);
    return mix(mix(x00, x10, w.y), mix(x01, x11, w.y), w.z);
}
