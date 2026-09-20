struct Patch {
    // xyz: patch origin relative to the camera.
    offset: vec4<f32>,
    // xyz: patch origin in planet space, wrapped to ANCHOR_M by the CPU in
    // f64. Detail is anchored here: same place, same phase, in every patch,
    // with no large f32 position ever built.
    anchor: vec4<f32>,
}

@group(1) @binding(0) var<uniform> placement: Patch;

// Must match `ANCHOR_M` in terrain.rs. Every detail scale divides it.
const ANCHOR_M: f32 = 1024.0;
const ROCK: vec3<f32> = vec3<f32>(0.115, 0.105, 0.098);

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
    @location(3) anchored: vec3<f32>,
}

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = in.position + placement.offset.xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    out.anchored = in.position + placement.anchor.xyz;
    return out;
}

// Noise at `cell_m` metres per cell, periodic over ANCHOR_M.
fn detail(p: vec3<f32>, cell_m: f32) -> f32 {
    return value_noise(p / cell_m, i32(ANCHOR_M / cell_m));
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    let distance = length(in.relative);
    let up = surface_up(in.relative);
    var normal = normalize(in.normal);

    // Three scales of detail; the fine ones fade before they would shimmer.
    let near = 1.0 - smoothstep(25.0, 90.0, distance);
    let mid = 1.0 - smoothstep(250.0, 900.0, distance);
    let fine = detail(in.anchored, 0.5);
    let medium = detail(in.anchored, 4.0);
    let coarse = detail(in.anchored, 32.0);

    // Bump from the detail height, on the unparametrised surface (Mikkelsen).
    let height = (fine * 0.035 + medium * 0.18) * near + coarse * 0.6 * mid;
    let dpdx_ = dpdx(in.relative);
    let dpdy_ = dpdy(in.relative);
    let r1 = cross(dpdy_, normal);
    let r2 = cross(normal, dpdx_);
    let det = dot(dpdx_, r1);
    let gradient = (r1 * dpdx(height) + r2 * dpdy(height)) * sign(det);
    normal = normalize(abs(det) * normal - gradient);

    // Rock shows where the ground is steep: per pixel, so the same at every LOD.
    let slope = 1.0 - dot(normalize(in.normal), up);
    let rock = smoothstep(0.16, 0.34, slope + (medium - 0.5) * 0.10 * mid);
    let cover = pow(in.color.rgb, vec3<f32>(2.2));
    var albedo = mix(cover, ROCK, rock);
    albedo *= 0.78 + 0.22 * mix(1.0, fine * 2.0, near) * mix(1.0, medium * 1.4 + 0.3, mid) + 0.12 * (coarse - 0.5);

    // Wet sand darkens and shines near the waterline.
    // Only the planet has a sea: far from its sea level this is zero anyway.
    let shore = 1.0 - smoothstep(0.0, 1.2, abs(length(view.camera.xyz + in.relative) - view.camera.w));
    albedo *= 1.0 - 0.35 * shore;
    let gloss = max(in.color.a, shore * 0.5) * (1.0 - rock);

    return encode(lit_surface(albedo, normal, gloss, in.relative, normalize(in.normal)));
}
