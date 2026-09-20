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

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    // A point on the near plane (depth 1) is a direction from the camera.
    let near = view.relative_from_clip * vec4<f32>(in.ndc, 1.0, 1.0);
    let dir = normalize(near.xyz / near.w);

    let air = atmosphere(dir, 1.0e9);
    let toward_sun = max(dot(dir, view.sun.xyz), 0.0);
    let disc = smoothstep(0.99985, 0.99995, toward_sun) * 20.0;
    let glow = pow(toward_sun, 64.0) * 0.25;
    let space = vec3<f32>(1.0, 0.95, 0.85) * (disc + glow);
    return encode(space * air.a + air.rgb);
}
