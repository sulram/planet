// The clouds stage: marches the weather between the camera and whatever the
// scene drew, from the ground, from inside the layer and from orbit alike.
// Light is single scattering toward the sun with a short march of its own,
// a two lobed phase (silver lining ahead, soft return behind), the powder
// darkening of thin edges, and sky light from above.

// Samples a ray may spend.
const CLOUD_SAMPLES: i32 = 128;
// A short step is this much of the distance it is taken at, and never under
// `CLOUD_STEP_M`: a far cloud covers few pixels and needs few samples.
const CLOUD_STEP_PER_M: f32 = 0.011;
const CLOUD_STEP_M: f32 = 18.0;
// Nor over this, or a stride could cross the whole layer between samples.
const CLOUD_STEP_MAX_M: f32 = 140.0;
const CLOUD_SUN_STEPS: i32 = 5;
// Extinction per metre of full density.
const CLOUD_EXTINCTION: f32 = 0.045;
const CLOUD_MARCH_M: f32 = 28000.0;

fn henyey_greenstein(cos_angle: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1.0 - g2) / (12.566371 * pow(1.0 + g2 - 2.0 * g * cos_angle, 1.5));
}

// Optical depth from a point toward the sun: steps that lengthen, since what
// is near shades most.
fn cloud_sun_depth(from_planet: vec3<f32>) -> f32 {
    var depth = 0.0;
    var at = 0.0;
    var step = 35.0;
    for (var i = 0; i < CLOUD_SUN_STEPS; i++) {
        at += step;
        depth += cloud_density(from_planet + view.sun.xyz * at, false) * step;
        step *= 1.8;
    }
    return depth * CLOUD_EXTINCTION * view.post.w;
}

// Stable per pixel jitter (interleaved gradient noise): trades banding for grain.
fn cloud_jitter(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

// rgb: light scattered toward the camera. a: how much of the scene survives.
// x of `span`: where the march starts, y: where it ends, metres along `dir`.
fn cloud_march(dir: vec3<f32>, span: vec2<f32>, jitter: f32, carry: vec4<f32>) -> vec4<f32> {
    let length_m = min(span.y - span.x, CLOUD_MARCH_M);
    if length_m <= 0.0 || carry.a < 0.01 {
        return carry;
    }
    // Two paces: long strides through clear air on the cheap field, and on
    // finding cloud, one stride back and short steps until it is left behind.
    var fine = clamp(span.x * CLOUD_STEP_PER_M, CLOUD_STEP_M, CLOUD_STEP_MAX_M);
    var stride = fine * 3.0;

    // The sun's colour where this span begins: warm when it is low over there.
    let place = normalize(view.camera.xyz + dir * span.x);
    let warm = 1.0 - smoothstep(-0.05, 0.35, dot(place, view.sun.xyz));
    let light = vec4<f32>(
        mix(vec3<f32>(1.0, 0.96, 0.90), SUNSET * vec3<f32>(1.0, 0.8, 0.7), warm),
        daylight(place));
    let toward_sun = dot(dir, view.sun.xyz);
    let phase = mix(henyey_greenstein(toward_sun, 0.78), henyey_greenstein(toward_sun, -0.28), 0.45);

    var scattered = carry.rgb;
    var through = carry.a;
    let end = span.x + length_m;
    var t = span.x + stride * jitter;
    var clear = 0;
    var inside = false;
    for (var i = 0; i < CLOUD_SAMPLES; i++) {
        if t >= end || through < 0.01 {
            break;
        }
        let relative = dir * t;
        let from_planet = view.camera.xyz + relative;
        fine = clamp(t * CLOUD_STEP_PER_M, CLOUD_STEP_M, CLOUD_STEP_MAX_M);
        stride = fine * 3.0;
        if !inside {
            if cloud_density(from_planet, false) > 0.0 {
                inside = true;
                clear = 0;
                t = max(t - stride + fine * jitter, span.x);
            } else {
                t += stride;
            }
            continue;
        }
        let density = cloud_density(from_planet, true);
        if density > 0.0 {
            clear = 0;
            let h = cloud_height(from_planet);
            let sun_depth = cloud_sun_depth(from_planet);
            // Beer for the way in, powder for the edges that face the viewer.
            let beer = max(exp(-sun_depth), 0.7 * exp(-sun_depth * 0.25));
            let powder = 1.0 - exp(-density * 9.0 - sun_depth * 2.0);
            let sun = light.rgb * (beer * mix(1.0, powder, 0.6) * phase * 28.0) * sunlight(relative);
            // Sky from above, the dim ground from below.
            let sky = (SKY * 0.55 + 0.10) * mix(0.35, 1.0, h) * light.a
                + vec3<f32>(0.010, 0.013, 0.022);
            let source = sun + sky;

            let extinction = density * CLOUD_EXTINCTION * view.post.w;
            let survive = exp(-extinction * fine);
            // Energy conserving: what this step adds, shaded by itself.
            scattered += through * source * (1.0 - survive);
            through *= survive;
        } else {
            clear += 1;
            inside = clear < 6;
        }
        t += fine;
    }
    return vec4<f32>(scattered, through);
}

// Where a ray meets the layer. The layer is a shell: a ray crosses it once,
// or twice around its hollow. Spans are metres along the ray, x to y.
struct CloudSpans {
    first: vec2<f32>,
    second: vec2<f32>,
    // Where the first cloud could be at all.
    enter: f32,
    any: bool,
}

fn cloud_spans(dir: vec3<f32>) -> CloudSpans {
    var spans: CloudSpans;
    let origin = view.camera.xyz;
    let outer = ray_sphere(origin, dir, view.camera.w + CLOUD_TOP_M);
    spans.any = outer.x <= outer.y && outer.y > 0.0;
    if !spans.any {
        return spans;
    }
    let inner = ray_sphere(origin, dir, view.camera.w + CLOUD_BASE_M);
    spans.first = vec2<f32>(max(outer.x, 0.0), outer.y);
    spans.second = vec2<f32>(0.0, 0.0);
    if inner.x <= inner.y && inner.y > 0.0 {
        spans.first.y = inner.x;
        spans.second = vec2<f32>(inner.y, outer.y);
    }
    spans.enter = select(spans.second.x, spans.first.x, spans.first.y > spans.first.x);
    return spans;
}

fn screen_dir(ndc: vec2<f32>) -> vec3<f32> {
    let near = view.relative_from_clip * vec4<f32>(ndc, 1.0, 1.0);
    return normalize(near.xyz / near.w);
}

// How far the scene lets a ray go. Depth is reversed: 0 is the sky.
fn scene_limit(ndc: vec2<f32>, dir: vec3<f32>, depth: f32) -> f32 {
    var limit = 1.0e9;
    if depth > 0.0 {
        let hit = view.relative_from_clip * vec4<f32>(ndc, depth, 1.0);
        limit = length(hit.xyz / hit.w);
    }
    // Far terrain may be culled: the planet itself stops the ray too.
    let ground = ray_sphere(view.camera.xyz, dir, view.camera.w);
    if ground.x <= ground.y && ground.x > 0.0 {
        limit = min(limit, ground.x);
    }
    return limit;
}

// Half size. rgb: cloud light toward the camera, hazed by the air between.
// a: how much of the scene survives.
@fragment
fn fs_march(in: Screen) -> @location(0) vec4<f32> {
    let clear = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    let dir = screen_dir(in.ndc);
    var spans = cloud_spans(dir);
    if !spans.any {
        return clear;
    }
    // The farthest of the four scene pixels under this one: a cloud must
    // reach behind a ridge, and the lay stage cuts it where the ridge is.
    let corner = vec2<i32>(in.clip.xy) * 2;
    let last = vec2<i32>(textureDimensions(stage_depth)) - 1;
    var depth = 1.0;
    for (var i = 0; i < 4; i++) {
        let at = min(corner + vec2<i32>(i & 1, i >> 1), last);
        depth = min(depth, textureLoad(stage_depth, at, 0));
    }
    let limit = scene_limit(in.ndc, dir, depth);
    spans.first.y = min(spans.first.y, limit);
    spans.second = vec2<f32>(spans.second.x, min(spans.second.y, limit));

    let jitter = cloud_jitter(in.clip.xy);
    var cloud = cloud_march(dir, spans.first, jitter, clear);
    cloud = cloud_march(dir, spans.second, jitter, cloud);
    // Air between the camera and the cloud hazes it like any far thing.
    let air = atmosphere(dir, spans.enter + 400.0);
    // From under the sea this runs before the surface is drawn, which bends
    // and tints the clouds with the rest of the world above.
    return vec4<f32>(cloud.rgb * air.a + air.rgb * (1.0 - cloud.a), cloud.a);
}

// Full size: the marched clouds over the scene, cut by what stands in front.
@fragment
fn fs_lay(in: Screen) -> @location(0) vec4<f32> {
    let pixel = stage_pixel(in);
    let scene = textureLoad(stage_color, pixel, 0);
    let dir = screen_dir(in.ndc);
    let spans = cloud_spans(dir);
    if !spans.any {
        return scene;
    }
    let limit = scene_limit(in.ndc, dir, textureLoad(stage_depth, pixel, 0));
    if limit <= spans.enter {
        return scene;
    }
    let uv = in.ndc * vec2<f32>(0.5, -0.5) + 0.5;
    let cloud = textureSampleLevel(stage_aux, stage_sampler, uv, 0.0);
    return vec4<f32>(scene.rgb * cloud.a + cloud.rgb, 1.0);
}
