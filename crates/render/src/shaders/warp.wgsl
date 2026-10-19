// A body's passage through a portal: the picture pulled into a point of
// the screen and streaked along the pull as the body falls in, a hot ring
// widening from the point and the rest going dark, the body itself left
// in sight, charged white; then, past the portal, the same let go: the
// picture bursting out of the point, the ring closing, the light coming
// back. Phase -1 to 0 is the fall, 0 to 1 the landing.

// How many samples a streak is made of.
const STREAK: u32 = 16u;
// How far the picture is pulled at most, of the distance to the point.
const PULL: f32 = 2.5;
// The ring at its widest, in heights of the viewport, and its width.
const RING: f32 = 0.32;
const RIM: f32 = 0.02;

@fragment
fn fs(in: Screen) -> @location(0) vec4<f32> {
    let uv = in.ndc * vec2<f32>(0.5, -0.5) + 0.5;
    let size = vec2<f32>(textureDimensions(stage_color));
    let aspect = size.x / size.y;
    let centre = view.warp.xy;
    let phase = view.warp.z;
    // How deep into the passage, 0 to 1 either way: rising on the way in,
    // falling on the way out.
    let depth = select(1.0 - phase, 1.0 + phase, phase < 0.0);
    let eased = depth * depth;
    let away = (uv - centre) * vec2<f32>(aspect, 1.0);
    let dist = length(away);
    // The pull: what shows at a pixel comes from further out along the
    // line to the point, and is streaked along it.
    let pull = PULL * eased;
    let flat = vec2<f32>(1.0 / aspect, 1.0);
    var color = vec3<f32>(0.0);
    for (var i = 0u; i < STREAK; i++) {
        let share = f32(i) / f32(STREAK - 1u);
        let scale = 1.0 + pull * share * (0.4 + 0.6 * smoothstep(0.0, 0.5, dist));
        // Colours part a little along the streak: the light is bent.
        let r = clamp(centre + away * scale * (1.0 + 0.03 * eased) * flat, vec2<f32>(0.0), vec2<f32>(1.0));
        let g = clamp(centre + away * scale * flat, vec2<f32>(0.0), vec2<f32>(1.0));
        let b = clamp(centre + away * scale * (1.0 - 0.03 * eased) * flat, vec2<f32>(0.0), vec2<f32>(1.0));
        color += vec3<f32>(
            textureSample(stage_color, stage_sampler, r).r,
            textureSample(stage_color, stage_sampler, g).g,
            textureSample(stage_color, stage_sampler, b).b,
        );
    }
    color /= f32(STREAK);
    // The ring, and the rest going dark as it widens.
    let ring = RING * eased;
    let rim = exp(-pow((dist - ring) / RIM, 2.0)) * eased;
    let dark = 1.0 - eased * eased * 0.8;
    color = color * dark + vec3<f32>(4.0, 3.5, 3.0) * rim;
    return vec4<f32>(color, 1.0);
}
