// The sea: its own surface, drawn after the opaque world and the sky, reading
// a copy of both and the depth. Seen from above it refracts the floor and takes
// its colour by the water actually crossed; seen from below the world above
// shows straight through it, rippled, and it mirrors only at a glancing look.

struct Patch {
    offset: vec4<f32>,
    anchor: vec4<f32>,
}

@group(1) @binding(0) var<uniform> placement: Patch;

// What lies behind the sea (`compose::Composer::behind`).
@group(2) @binding(0) var behind_color: texture_2d<f32>;
@group(2) @binding(1) var behind_depth: texture_depth_2d;
@group(2) @binding(2) var behind_sampler: sampler;

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

// Where a point given from the camera lands in the copy of the scene.
fn behind_uv(relative: vec3<f32>) -> vec2<f32> {
    let clip = view.clip_from_relative * vec4<f32>(relative, 1.0);
    return clip.xy / clip.w * vec2<f32>(0.5, -0.5) + 0.5;
}

// How far the scene is along the ray through a place in the copy, metres.
fn behind_distance(uv: vec2<f32>) -> f32 {
    let size = vec2<f32>(textureDimensions(behind_depth));
    let pixel = clamp(vec2<i32>(uv * size), vec2<i32>(0), vec2<i32>(size) - 1);
    let depth = textureLoad(behind_depth, pixel, 0);
    if depth <= 0.0 {
        return 1.0e9;
    }
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let hit = view.relative_from_clip * vec4<f32>(ndc, depth, 1.0);
    return length(hit.xyz / hit.w);
}

// Whether what the copy holds at a place lies past the sea's surface along its
// own ray: only that may be seen through the surface. An avatar swimming in
// front of it may not, or its ghost is bent into the picture.
fn past_surface(uv: vec2<f32>) -> bool {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let near = view.relative_from_clip * vec4<f32>(ndc, 1.0, 1.0);
    let sea = ray_sphere(view.camera.xyz, normalize(near.xyz / near.w), view.camera.w);
    // From under it the surface is where the ray leaves the sphere; from over
    // it, where it enters.
    let surface = select(sea.x, sea.y, view.flags.z < 0.0);
    return sea.x <= sea.y && behind_distance(uv) > surface;
}

fn on_screen(uv: vec2<f32>) -> bool {
    return all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
}

@fragment
fn fs(in: Varying, @builtin(front_facing) from_above: bool) -> @location(0) vec4<f32> {
    // Dry land pokes through the grid: nothing to draw there.
    if in.depth_m < -0.05 {
        discard;
    }
    let distance = length(in.relative);
    // This pass has no depth of its own: whatever the scene drew nearer wins.
    let here = in.clip.xy / vec2<f32>(textureDimensions(behind_depth));
    let scene_m = behind_distance(here);
    if scene_m < distance {
        discard;
    }
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
        // From below: the world above, straight through the surface, where a
        // swimmer looks for it, rippled by the swell. Only at a glancing look
        // does the surface turn into a mirror of the sea. (The physical bend
        // folds the whole horizon into a cone overhead: true, and it put the
        // shore in the sky.)
        let window = snell_window(facing);
        let mirror = WATER_SCATTER * (0.35 + 1.6 * day);
        let ripple = (normal - up * dot(normal, up)) * min(scene_m - distance, 6.0) * 0.5;
        var uv = behind_uv(in.relative + dir * min(scene_m - distance, 6.0) + ripple);
        // Never a swimmer in front of the surface, nor off the picture.
        if !on_screen(uv) || !past_surface(uv) {
            uv = here;
        }
        let size = vec2<f32>(textureDimensions(behind_color));
        let texel = clamp(vec2<i32>(uv * size), vec2<i32>(0), vec2<i32>(size) - 1);
        // The world above came with its air alone; the water between is here.
        let above = textureLoad(behind_color, texel, 0).rgb;
        return vec4<f32>(through_water(mix(mirror, above, window), distance), 1.0);
    }
    // From above. What lies under the surface is seen bent, the more the
    // deeper it lies, and never from in front of the surface.
    let reach = clamp(scene_m - distance, 0.0, 30.0);
    let tangent = normal - up * dot(normal, up);
    var uv = behind_uv(in.relative + dir * reach + tangent * reach * 0.6);
    var floor_m = behind_distance(uv);
    if !on_screen(uv) || !past_surface(uv) {
        uv = here;
        floor_m = scene_m;
    }
    let floor_color = textureSampleLevel(behind_color, behind_sampler, uv, 0.0).rgb;

    // The sea takes light as the land does: what shades a meadow shades a bay.
    let sun_reach = sunlight(in.relative) * terrain_shadow(in.relative, up) * cloud_shadow(in.relative);
    // The sun lights what the water scatters back; the sky lights it too.
    let body = WATER_SCATTER * (0.10 + day * (0.40 + 0.85 * sun_reach));
    // Light that reaches the floor and comes back crosses the water twice:
    // down to it, by the depth under this place, and back along the ray.
    // Each colour dies at its own rate.
    let path = min(max(floor_m - distance, 0.0) + max(in.depth_m, 0.0), 400.0);
    let survive = exp(-WATER_ABSORB * path);
    // The floor came hazed by the air as far as itself. What the water adds of
    // its own is hazed here, or a far shallow glows through the distance.
    let air = atmosphere(dir, distance);
    let under = floor_color * survive + (body * air.a + air.rgb) * (1.0 - survive);

    // Fresnel: the sky where you look along the surface, the water where you
    // look down into it.
    let fresnel = 0.02 + 0.98 * pow(1.0 - facing, 5.0);
    let mirrored = reflect(dir, normal);
    let sky = atmosphere(mirrored, 1.0e9).rgb + SKY * 0.08 * day;
    let glint = pow(max(dot(mirrored, view.sun.xyz), 0.0), 900.0) * 1.6 * day * sun_reach;

    // Foam where the water runs out. Coarse patches far away cannot resolve a
    // shoreline: no foam there.
    let lap = 0.5 + 0.5 * sin(in.depth_m * 9.0 - t * 1.6 + h * 8.0);
    let foam = (1.0 - smoothstep(0.0, 0.55, in.depth_m)) * (0.35 + 0.65 * lap) * (0.4 + 0.6 * day) * calm;

    var surface = sky * fresnel + glint;
    surface = mix(surface, vec3<f32>(0.92) * mix(0.55, 1.0, sun_reach), foam);
    let own = surface * air.a + air.rgb * max(fresnel, foam);
    return vec4<f32>(under * (1.0 - fresnel) * (1.0 - foam) + own, 1.0);
}
