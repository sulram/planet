// The weather: how much cloud there is at a point around the planet. Two
// readers: the clouds stage marches it, lit surfaces ask it for shade.
@group(0) @binding(3) var cloud_noise: texture_3d<f32>;
@group(0) @binding(4) var cloud_sampler: sampler;

// Metres across one repeat of the noise as weather. Bodies and wisps have
// theirs in the prelude: the CPU raises the noise by repeats of them.
const CLOUD_WEATHER_TILE_M: f32 = 38000.0;

fn cloud_remap(x: f32, low: f32, high: f32) -> f32 {
    return clamp((x - low) / (high - low), 0.0, 1.0);
}

// 0 at the base of the layer, 1 at its top. Outside is outside.
fn cloud_height(from_planet: vec3<f32>) -> f32 {
    return (length(from_planet) - view.camera.w - CLOUD_BASE_M) / (CLOUD_TOP_M - CLOUD_BASE_M);
}

// Density at a point given from the planet's centre, 0 to 1. `detail` carves
// the edges: the march wants it, a shadow or a step toward the sun does not.
fn cloud_density(from_planet: vec3<f32>, detail: bool) -> f32 {
    let h = cloud_height(from_planet);
    if h <= 0.0 || h >= 1.0 {
        return 0.0;
    }
    // The weather turns about the axis: sample where this air came from.
    let wind = view.clouds.xy;
    let p = vec3<f32>(
        from_planet.x * wind.x + from_planet.z * wind.y,
        from_planet.y,
        from_planet.z * wind.x - from_planet.x * wind.y);

    let weather = textureSampleLevel(cloud_noise, cloud_sampler, p / CLOUD_WEATHER_TILE_M, 0.0).a;
    // Cover moves where the weather begins: less of it, fewer places have any.
    // The smooth noise stays near its middle, 0.3 to 0.7: the window sits there.
    let open = 0.64 - 0.26 * view.clouds.z;
    let cover = smoothstep(open, open + 0.18, weather);
    if cover <= 0.0 {
        return 0.0;
    }
    // The noise rises through the layer: a heap boils and reshapes where it
    // stands, while the wind carries it.
    let up = normalize(p);
    let noise = textureSampleLevel(cloud_noise, cloud_sampler, p / CLOUD_SHAPE_TILE_M - up * view.post.y, 0.0);
    let billow = noise.g * 0.625 + noise.b * 0.375;
    let body = cloud_remap(noise.r, billow - 1.0, 1.0);
    // The cut rises with height: only the strongest of the noise stands tall,
    // so a heap narrows into a dome of its own height instead of meeting a
    // ceiling. The weather sets the cut at the base: scattered heaps where it
    // is fair, a closing deck where it is full.
    let base_cut = mix(0.88, 0.60, cover);
    let cut = base_cut + (1.04 - base_cut) * pow(h, 0.8);
    var density = cloud_remap(body, cut, cut + 0.20) * smoothstep(0.0, 0.07, h);
    if density <= 0.0 || !detail {
        return density;
    }
    let fine = textureSampleLevel(cloud_noise, cloud_sampler, p / CLOUD_DETAIL_TILE_M - up * view.post.z, 0.0);
    // Wisps below, where air is drawn in; billows above, where it boils out.
    let wisp = fine.g * 0.6 + fine.b * 0.4;
    let erosion = mix(wisp, 1.0 - wisp, clamp(h * 4.0, 0.0, 1.0));
    return cloud_remap(density, erosion * 0.28, 1.0);
}

// How much sun reaches a surface point under the weather, 0 to 1. One sample
// where the ray to the sun crosses the middle of the layer.
fn cloud_shadow(relative: vec3<f32>) -> f32 {
    if view.clouds.w < 0.5 {
        return 1.0;
    }
    let from_planet = view.camera.xyz + relative;
    let middle = view.camera.w + mix(CLOUD_BASE_M, CLOUD_TOP_M, 0.2);
    if dot(from_planet, from_planet) >= middle * middle {
        return 1.0;
    }
    let hit = ray_sphere(from_planet, view.sun.xyz, middle);
    let density = cloud_density(from_planet + view.sun.xyz * hit.y, false);
    return exp(-density * 5.0 * view.post.w);
}
