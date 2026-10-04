// A guide: a box of cells drawn as a faint grid over the world, so a hand
// sees where cells are before any is laid. Its own light, dimmed by night.

struct Placement {
    // xyz: mesh origin relative to the camera.
    offset: vec4<f32>,
    // Unused here; the slot is the one terrain binds.
    anchor: vec4<f32>,
}

@group(1) @binding(0) var<uniform> placement: Placement;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) lattice: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) lattice: vec2<f32>,
    @location(2) color: vec4<f32>,
}

// How many times as much of a guide shows on a line as between two.
const LINE: f32 = 6.0;
// Cells between two lines drawn stronger: a bay of a deck, and half a chunk.
const BAY: f32 = 8.0;

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = in.position + placement.offset.xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.lattice = in.lattice;
    out.color = in.color;
    return out;
}

// How much of a line a pixel wide lies over this one, on a lattice whose
// lines are `every` cells apart. Lines that crowd closer than a few pixels
// fade out, each way on its own, so a grid far off or seen edge on is its
// faint fill and never a moire.
fn lines(at: vec2<f32>, every: f32) -> f32 {
    let cells = at / every;
    let pixel = max(fwidth(cells), vec2<f32>(1e-6));
    let away = abs(fract(cells - 0.5) - 0.5) / pixel;
    let line = (1.0 - clamp(away, vec2<f32>(0.0), vec2<f32>(1.0)))
        * (1.0 - smoothstep(vec2<f32>(0.12), vec2<f32>(0.4), pixel));
    return max(line.x, line.y);
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    let line = max(lines(in.lattice, 1.0), lines(in.lattice, BAY) * 2.0);
    let alpha = clamp(in.color.a * (1.0 + (LINE - 1.0) * min(line, 2.0)), 0.0, 0.9);
    // Lit by nothing, and no brighter than the day around it.
    let day = sunlight(in.relative) * daylight(surface_up(in.relative));
    let color = pow(in.color.rgb, vec3<f32>(2.2)) * mix(0.12, 1.0, day);
    let distance = length(in.relative);
    return vec4<f32>(through_medium(color, in.relative / distance, distance), alpha);
}
