// The last stage: exposure, the filmic curve, and the encoding the target
// wants. The only place linear light becomes a picture.

// Filmic curve (Narkowicz's fit of ACES): highlights roll off instead of
// clipping. Contrasty and saturated; very bright colours skew in hue.
fn tone_aces(x: vec3<f32>) -> vec3<f32> {
    return (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
}

// AgX (Troy Sobotka), in its compact form: into a wide gamut, a log encoding
// over 16.5 stops, a sigmoid, and back. Bright colours fade toward white.
fn tone_agx(x: vec3<f32>) -> vec3<f32> {
    let inset = mat3x3<f32>(
        0.842479062253094, 0.0423282422610123, 0.0423756549057051,
        0.0784335999999992, 0.878468636469772, 0.0784336,
        0.0792237451477643, 0.0791661274605434, 0.879142973793104);
    let outset = mat3x3<f32>(
        1.19687900512017, -0.0528968517574562, -0.0529716355144438,
        -0.0980208811401368, 1.15190312990417, -0.0980434501171241,
        -0.0990297440797205, -0.0989611768448433, 1.15107367264116);
    let encoded = clamp((log2(max(inset * x, vec3<f32>(1.0e-10))) + 12.47393) / 16.5, vec3<f32>(0.0), vec3<f32>(1.0));
    let e2 = encoded * encoded;
    let e4 = e2 * e2;
    let curved = 15.5 * e4 * e2 - 40.14 * e4 * encoded + 31.96 * e4 - 6.868 * e2 * encoded
        + 0.4298 * e2 + 0.1191 * encoded - 0.00232;
    // The curve answers in display space; the rest of this stage is linear.
    return pow(max(outset * curved, vec3<f32>(0.0)), vec3<f32>(2.2));
}

// Khronos PBR neutral: below 0.8 a colour is as authored; above, only the
// peak is compressed and the colour eases toward white.
fn tone_neutral(color_in: vec3<f32>) -> vec3<f32> {
    let start = 0.8 - 0.04;
    let desaturation = 0.15;
    var color = color_in;
    let low = min(color.r, min(color.g, color.b));
    let offset = select(0.04, low - 6.25 * low * low, low < 0.08);
    color -= offset;
    let peak = max(color.r, max(color.g, color.b));
    if peak < start {
        return color;
    }
    let d = 1.0 - start;
    let compressed = 1.0 - d * d / (peak + d - start);
    color *= compressed / peak;
    let white = 1.0 - 1.0 / (desaturation * (peak - compressed) + 1.0);
    return mix(color, vec3<f32>(compressed), white);
}

// Reinhard on luminance, with a white point: plain, and flat beside the rest.
fn tone_reinhard(x: vec3<f32>) -> vec3<f32> {
    let white = 4.0;
    let l = dot(x, vec3<f32>(0.2126, 0.7152, 0.0722));
    let mapped = l * (1.0 + l / (white * white)) / (1.0 + l);
    return x * (mapped / max(l, 1.0e-5));
}

// The order of `scene::ToneMap::ALL`.
fn tone_map(x: vec3<f32>) -> vec3<f32> {
    var mapped = x;
    switch u32(view.grade.y) {
        case 0u: { mapped = tone_aces(x); }
        case 1u: { mapped = tone_agx(x); }
        case 2u: { mapped = tone_neutral(x); }
        case 3u: { mapped = tone_reinhard(x); }
        default: {}
    }
    return clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0));
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
