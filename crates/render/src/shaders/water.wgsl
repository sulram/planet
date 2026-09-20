// The sea: its own surface over the terrain, blended, seen from both sides.

struct Patch {
    offset: vec4<f32>,
    anchor: vec4<f32>,
}

@group(1) @binding(0) var<uniform> placement: Patch;

const ANCHOR_M: f32 = 1024.0;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) depth_m: f32,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) anchored: vec3<f32>,
    @location(2) depth_m: f32,
}

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = in.position + placement.offset.xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.anchored = in.position + placement.anchor.xyz;
    out.depth_m = in.depth_m;
    return out;
}

// Height of the swell at a place and time: two drifting layers of noise.
fn swell(p: vec3<f32>, t: f32) -> f32 {
    let a = value_noise((p + vec3<f32>(t * 0.9, t * 0.3, t * 0.6)) / 4.0, 256);
    let b = value_noise((p - vec3<f32>(t * 0.5, t * 0.8, -t * 0.4)) / 1.0, 1024);
    // Chop: what breaks the sun's reflection into sparkle instead of blobs.
    let c = value_noise((p + vec3<f32>(-t * 0.7, t * 0.5, t * 0.9)) / 0.25, 4096);
    return a * 0.55 + b * 0.10 + c * 0.018;
}

@fragment
fn fs(in: Varying, @builtin(front_facing) from_above: bool) -> @location(0) vec4<f32> {
    // Dry land pokes through the grid: nothing to draw there.
    if in.depth_m < -0.05 {
        discard;
    }
    let distance = length(in.relative);
    let dir = in.relative / distance;
    let up = normalize(view.camera.xyz + in.relative);
    let t = view.flags.y;

    // Normal from the slope of the swell along the two surface directions.
    let calm = 1.0 - smoothstep(60.0, 400.0, distance);
    let east = normalize(cross(up, vec3<f32>(0.0, 1.0, 0.0)) + vec3<f32>(1e-4, 0.0, 0.0));
    let north = cross(east, up);
    let h = swell(in.anchored, t);
    let hx = swell(in.anchored + east * 0.25, t);
    let hy = swell(in.anchored + north * 0.25, t);
    var normal = normalize(up - (east * (hx - h) + north * (hy - h)) * 4.0 * calm);

    let day = daylight(up);
    let facing = abs(dot(normal, dir));
    if !from_above {
        // From below. Inside Snell's window (about 48 degrees from straight
        // up) the world above shows through; outside it the surface is a
        // mirror of the sea (total internal reflection).
        let window = smoothstep(0.62, 0.74, facing);
        let mirror = WATER_SCATTER * (0.10 + 1.3 * day);
        let ceiling = vec3<f32>(0.55, 0.85, 0.90) * day;
        let color = mix(mirror, ceiling, window);
        let seen = through_medium(color, dir, distance);
        return vec4<f32>(seen, mix(0.97, 0.22, window));
    }

    // From above. Light that reaches the floor and comes back crosses the
    // water twice, further at a glancing angle; each colour dies at its rate.
    let path = max(in.depth_m, 0.0) * (1.0 + 1.0 / max(facing, 0.2));
    let survive = exp(-WATER_ABSORB * path);
    let body = WATER_SCATTER * (0.10 + 1.25 * day);

    // Fresnel: the sky where you look along the surface, the water where you
    // look down into it.
    let fresnel = 0.02 + 0.98 * pow(1.0 - facing, 5.0);
    let mirrored = reflect(dir, normal);
    let sky = atmosphere(mirrored, 1.0e9).rgb + SKY * 0.08 * day;
    let glint = pow(max(dot(mirrored, view.sun.xyz), 0.0), 900.0) * 1.6 * day;

    // Foam where the water runs out. Coarse patches far away cannot resolve a
    // shoreline: no foam there.
    let lap = 0.5 + 0.5 * sin(in.depth_m * 9.0 - t * 1.6 + h * 8.0);
    let foam = (1.0 - smoothstep(0.0, 0.55, in.depth_m)) * (0.35 + 0.65 * lap) * (0.4 + 0.6 * day) * calm;

    var color = mix(body, sky, fresnel) + glint;
    color = mix(color, vec3<f32>(0.92), foam);
    // What survives the round trip is the floor showing through. One alpha
    // cannot tint per colour, so it follows green, the middle of the three.
    let alpha = clamp(max(max(1.0 - survive.g, fresnel), foam), 0.0, 1.0);
    let seen = through_medium(color, dir, distance);
    return vec4<f32>(seen, alpha);
}
