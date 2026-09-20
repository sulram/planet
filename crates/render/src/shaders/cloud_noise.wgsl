// The stuff clouds are made of: one tiling 3D texture, drawn a slice at a time.
//   r: Perlin-Worley, the billowing body of a cloud
//   g, b: Worley at rising frequency, what carves its edges
//   a: smooth low noise, the weather: where clouds are at all
// Every period is an integer count of cells across the texture, so it tiles.

struct Slice { at: vec4<f32> }
@group(1) @binding(0) var<uniform> slice: Slice;

struct Screen {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> Screen {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: Screen;
    out.clip = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    out.uv = corner;
    return out;
}

// Three numbers per cell of a lattice that repeats every `period` cells.
fn lattice3(cell: vec3<i32>, period: i32) -> vec3<f32> {
    let c = vec3<u32>((cell % period + period) % period);
    var h = (c.x * 0x8da6b343u) ^ (c.y * 0xd8163841u) ^ (c.z * 0xcb1ab31fu);
    h = (h ^ (h >> 15u)) * 0x2c1b3c6du;
    h = (h ^ (h >> 12u)) * 0x297a2d39u;
    h = h ^ (h >> 15u);
    return vec3<f32>(vec3<u32>(h, h >> 10u, h >> 20u) & vec3<u32>(0x3ffu)) / 1023.0;
}

// 1 at a feature point, falling to 0 half a cell away: inverted cellular noise.
fn worley(p: vec3<f32>, cells: i32) -> f32 {
    let scaled = p * f32(cells);
    let cell = vec3<i32>(floor(scaled));
    let f = fract(scaled);
    var nearest = 1.0;
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let o = vec3<i32>(x, y, z);
                let d = vec3<f32>(o) + lattice3(cell + o, cells) - f;
                nearest = min(nearest, dot(d, d));
            }
        }
    }
    return 1.0 - clamp(sqrt(nearest), 0.0, 1.0);
}

fn worley_fbm(p: vec3<f32>, cells: i32) -> f32 {
    return worley(p, cells) * 0.625 + worley(p, cells * 2) * 0.25 + worley(p, cells * 4) * 0.125;
}

fn smooth_fbm(p: vec3<f32>, cells: i32) -> f32 {
    var sum = 0.0;
    var weight = 0.5;
    var period = cells;
    for (var octave = 0; octave < 4; octave++) {
        sum += value_noise(p * f32(period), period) * weight;
        weight *= 0.5;
        period *= 2;
    }
    return sum / 0.9375;
}

fn remap(x: f32, low: f32, high: f32, to_low: f32, to_high: f32) -> f32 {
    return to_low + (x - low) / (high - low) * (to_high - to_low);
}

@fragment
fn fs(in: Screen) -> @location(0) vec4<f32> {
    let p = vec3<f32>(in.uv, slice.at.x);
    let billow = worley_fbm(p, 4);
    // Smooth noise pushed out by the cells: connected, and round at the edge.
    let body = remap(smooth_fbm(p, 4), 0.0, 1.0, billow * 0.7, 1.0);
    return vec4<f32>(clamp(body, 0.0, 1.0), worley_fbm(p, 6), worley_fbm(p, 12), smooth_fbm(p, 3));
}
