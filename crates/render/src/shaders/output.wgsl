// The last stage: exposure, the filmic curve, and the encoding the target
// wants. The only place linear light becomes a picture.

// Filmic curve (Narkowicz's ACES fit): highlights roll off instead of clipping.
fn tone_map(x: vec3<f32>) -> vec3<f32> {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs(in: Screen) -> @location(0) vec4<f32> {
    let linear = textureLoad(stage_color, stage_pixel(in), 0).rgb;
    let c = tone_map(linear * view.post.x);
    // x: 1 when the target is not sRGB and the shader must encode.
    if view.flags.x < 0.5 {
        return vec4<f32>(c, 1.0);
    }
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return vec4<f32>(select(high, low, c <= vec3<f32>(0.0031308)), 1.0);
}
