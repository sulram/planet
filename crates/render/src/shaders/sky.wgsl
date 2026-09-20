// The sky: one triangle over the screen at the far plane (depth 0, reversed),
// drawn last so it only fills what the world left empty.

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> Varying {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: Varying;
    out.clip = vec4<f32>(corner, 0.0, 1.0);
    out.ndc = corner;
    return out;
}

// Stars: one candidate per cell of a grid on the direction cube, fixed to the
// world so the night sky turns with the planet's day.
fn stars(dir: vec3<f32>) -> vec3<f32> {
    let size = abs(dir);
    let longest = max(max(size.x, size.y), size.z);
    // 1 on the axis of the cube face this direction hits, 0 on the two axes
    // that run along the face. The grid is 2D on the face: the third
    // coordinate is pinned to the face and only names it.
    let across = step(vec3<f32>(longest), size);
    let grid = dir / longest * 90.0;
    let cell = vec3<i32>(mix(floor(grid), sign(dir) * 91.0, across));
    let chance = lattice(cell, 1024);
    if chance < 0.90 {
        return vec3<f32>(0.0);
    }
    let jitter = vec3<f32>(lattice(cell + vec3<i32>(7, 0, 0), 1024), lattice(cell + vec3<i32>(0, 7, 0), 1024), lattice(cell + vec3<i32>(0, 0, 7), 1024));
    let spot = length((fract(grid) - 0.25 - 0.5 * jitter) * (1.0 - across));
    let brightness = pow((chance - 0.90) * 10.0, 3.0);
    let radius = 0.05 + 0.10 * brightness;
    let tint = mix(vec3<f32>(1.0, 0.82, 0.65), vec3<f32>(0.72, 0.84, 1.0), jitter.x);
    // smoothstep wants its edges in order.
    return tint * (1.0 - smoothstep(0.0, radius, spot)) * (0.25 + 5.0 * brightness);
}

// The moon: a real sphere, hit by the view ray and shaded by the real sun, so
// it has phases from any side and fills the sky as you fly to it.
// rgb: its light, a: how much of the pixel it covers.
fn moon_disc(dir: vec3<f32>) -> vec4<f32> {
    let center = view.moon.xyz;
    let radius = view.moon.w;
    let along = dot(center, dir);
    let miss2 = dot(center, center) - along * along;
    // Derivatives must be taken before any branch (uniform control flow).
    let edge = sqrt(max(miss2, 0.0)) / radius;
    let rim = 1.5 * fwidth(edge);
    if along < 0.0 || miss2 > radius * radius {
        return vec4<f32>(0.0);
    }
    let hit = dir * (along - sqrt(radius * radius - miss2));
    let normal = (hit - center) / radius;
    // Soft rim: about a pixel wide, from orbit or standing on it.
    let cover = 1.0 - smoothstep(1.0 - rim, 1.0, edge);
    let lit = smoothstep(-0.02, 0.12, dot(normal, view.sun.xyz));
    // Maria and craters fixed to the surface, finer ones as it comes close.
    let face = normal * 4.0;
    let maria = value_noise(face + 40.0, 64);
    // Regolith shows only up close: finer grain as the surface nears.
    let near = 1.0 - smoothstep(200.0, 3000.0, length(hit));
    let grain = value_noise(normal * radius / 8.0, 4096) * 0.6 + value_noise(normal * radius / 0.7, 65536) * 0.4;
    let rough = value_noise(face * 6.0 + 9.0, 256) * 0.5 + value_noise(face * 40.0, 1024) * 0.25 + (grain - 0.5) * 1.1 * near;
    let albedo = 0.16 + 0.34 * smoothstep(0.35, 0.65, maria) - 0.16 * rough;
    let shine = vec3<f32>(0.95, 0.93, 0.86) * albedo * (lit * 1.05 + 0.010);
    return vec4<f32>(shine, cover);
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    // A point on the near plane (depth 1) is a direction from the camera.
    let near = view.relative_from_clip * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(near.xyz / near.w);

    // Under the sea there is no sky, only water all the way.
    if view.flags.z < 0.0 {
        return encode(through_medium(vec3<f32>(0.0), dir, 1.0e6));
    }

    let air = atmosphere(dir, 1.0e9);
    let toward_sun = max(dot(dir, view.sun.xyz), 0.0);
    let disc = smoothstep(0.99985, 0.99995, toward_sun) * 20.0;
    let glow = pow(toward_sun, 64.0) * 0.25;
    // The planet hides the stars behind it.
    let ground = ray_sphere(view.camera.xyz, dir, view.camera.w);
    let open_sky = select(1.0, 0.0, ground.x <= ground.y && ground.y > 0.0);
    // Daylight drowns the stars: the air along the ray outshines them, and so
    // does standing in daylight, however dark that corner of the sky is.
    let in_air = 1.0 - smoothstep(0.0, view.sun.w - view.camera.w, view.flags.z);
    let dazzle = (1.0 - smoothstep(0.02, 0.20, max(air.r, max(air.g, air.b))))
        * (1.0 - in_air * daylight(normalize(view.camera.xyz)));
    let sun = vec3<f32>(1.0, 0.95, 0.85) * (disc + glow);
    let moon = moon_disc(dir);
    // The moon hides the stars behind it, lit side or dark.
    let night_sky = mix(stars(dir) * dazzle, moon.rgb, moon.a);
    return encode((sun + night_sky * open_sky) * air.a + air.rgb);
}
