// Bloom: light above a threshold spills over its neighbours, as it does in an
// eye or a lens. The bright part of the scene is halved down a pyramid and
// summed back up it, so the glow is wide without a wide kernel.
// view.bloom: x strength, y threshold, z knee.

fn stage_uv(in: Screen) -> vec2<f32> {
    return in.ndc * vec2<f32>(0.5, -0.5) + 0.5;
}

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// A 13 tap downsample (Jimenez, Call of Duty: Advanced Warfare): five
// overlapping 2x2 boxes. `karis` weighs each box by its brightness, so one
// blazing pixel cannot flicker the whole glow: wanted on the first halving.
fn downsample(uv: vec2<f32>, karis: bool) -> vec3<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(stage_color));
    let a = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(-2.0, -2.0), 0.0).rgb;
    let b = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(0.0, -2.0), 0.0).rgb;
    let c = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(2.0, -2.0), 0.0).rgb;
    let d = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(-2.0, 0.0), 0.0).rgb;
    let e = textureSampleLevel(stage_color, stage_sampler, uv, 0.0).rgb;
    let f = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(2.0, 0.0), 0.0).rgb;
    let g = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(-2.0, 2.0), 0.0).rgb;
    let h = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(0.0, 2.0), 0.0).rgb;
    let i = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(2.0, 2.0), 0.0).rgb;
    let j = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(-1.0, -1.0), 0.0).rgb;
    let k = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(1.0, -1.0), 0.0).rgb;
    let l = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(-1.0, 1.0), 0.0).rgb;
    let m = textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(1.0, 1.0), 0.0).rgb;
    var boxes = array<vec3<f32>, 5>(
        (j + k + l + m) * 0.25,
        (a + b + d + e) * 0.25,
        (b + c + e + f) * 0.25,
        (d + e + g + h) * 0.25,
        (e + f + h + i) * 0.25);
    var weights = array<f32, 5>(0.5, 0.125, 0.125, 0.125, 0.125);
    var sum = vec3<f32>(0.0);
    var total = 0.0;
    for (var n = 0; n < 5; n++) {
        var weight = weights[n];
        if karis {
            weight /= 1.0 + luminance(boxes[n]);
        }
        sum += boxes[n] * weight;
        total += weight;
    }
    return sum / total;
}

// The scene, halved, keeping what is over the threshold. The knee eases it in:
// a hard cut makes a surface pop into glowing as it turns to the sun.
@fragment
fn fs_bright(in: Screen) -> @location(0) vec4<f32> {
    let color = min(downsample(stage_uv(in), true), vec3<f32>(64.0));
    let brightness = max(color.r, max(color.g, color.b));
    let knee = max(view.bloom.z, 0.0001);
    let soft = clamp(brightness - view.bloom.y + knee, 0.0, 2.0 * knee);
    let kept = max(soft * soft / (4.0 * knee), brightness - view.bloom.y);
    return vec4<f32>(color * (kept / max(brightness, 0.0001)), 1.0);
}

@fragment
fn fs_down(in: Screen) -> @location(0) vec4<f32> {
    return vec4<f32>(downsample(stage_uv(in), false), 1.0);
}

// A 3x3 tent over the smaller level, added to the larger one it is drawn on.
@fragment
fn fs_up(in: Screen) -> @location(0) vec4<f32> {
    let uv = stage_uv(in);
    let texel = 1.0 / vec2<f32>(textureDimensions(stage_color));
    var sum = vec3<f32>(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let weight = f32((2 - abs(x)) * (2 - abs(y))) / 16.0;
            sum += textureSampleLevel(stage_color, stage_sampler, uv + texel * vec2<f32>(f32(x), f32(y)), 0.0).rgb * weight;
        }
    }
    return vec4<f32>(sum, 1.0);
}

// The summed glow, over the scene. Divided by its levels, so strength reads the
// same whatever the depth of the pyramid.
@fragment
fn fs_lay(in: Screen) -> @location(0) vec4<f32> {
    let scene = textureLoad(stage_color, stage_pixel(in), 0).rgb;
    let glow = textureSampleLevel(stage_aux, stage_sampler, stage_uv(in), 0.0).rgb;
    return vec4<f32>(scene + glow * (view.bloom.x / f32(BLOOM_LEVELS)), 1.0);
}
