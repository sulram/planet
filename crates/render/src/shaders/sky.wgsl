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

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    // A point on the near plane (depth 1) is a direction from the camera.
    let near = view.relative_from_clip * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(near.xyz / near.w);

    // Under the sea there is no sky, only water all the way.
    if view.flags.z < 0.0 {
        return vec4<f32>(through_medium(vec3<f32>(0.0), dir, 1.0e6), 1.0);
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
    // The moon is terrain with depth of its own: it hides sun, stars and
    // planet by being in front. The planet's night side may be culled, so it
    // hides them here.
    return vec4<f32>((sun + stars(dir) * dazzle) * open_sky * air.a + air.rgb, 1.0);
}
