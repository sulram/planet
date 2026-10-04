// Guides and the ghost of a stroke: sides of cells drawn see-through over the
// world with a line between each cell and the next, so a hand sees where
// cells are and how many. Lit by nothing: a side is shaded by how it faces
// the sky, so a box reads as one, and it stays plain to see by night.

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
    @location(3) ink: vec4<f32>,
    @location(4) rim: vec4<f32>,
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) relative: vec3<f32>,
    @location(1) lattice: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) ink: vec4<f32>,
    @location(4) rim: vec4<f32>,
}

// Cells between two lines drawn whole: a bay of a deck, and half a chunk.
// The lines between are drawn half as strong.
const BAY: f32 = 8.0;

@vertex
fn vs(in: Vertex) -> Varying {
    var out: Varying;
    out.relative = in.position + placement.offset.xyz;
    out.clip = view.clip_from_relative * vec4<f32>(out.relative, 1.0);
    out.lattice = in.lattice;
    out.color = in.color;
    out.ink = in.ink;
    out.rim = in.rim;
    return out;
}

// How much of a line a pixel wide lies over this one, on a lattice whose
// lines are `every` cells apart, and how much of the rim that runs a pixel
// off it each side. Lines that crowd closer than a few pixels fade out,
// each way on its own, so a grid far off or seen edge on is its faint fill
// and never a moire.
fn lines(at: vec2<f32>, every: f32) -> vec2<f32> {
    let cells = at / every;
    let pixel = max(fwidth(cells), vec2<f32>(1e-6));
    let away = abs(fract(cells - 0.5) - 0.5) / pixel;
    let shown = 1.0 - smoothstep(vec2<f32>(0.12), vec2<f32>(0.4), pixel);
    let on = clamp(away, vec2<f32>(0.0), vec2<f32>(1.0));
    let line = (1.0 - on) * shown;
    let rim = (on - clamp(away - 1.0, vec2<f32>(0.0), vec2<f32>(1.0))) * shown;
    return vec2<f32>(max(line.x, line.y), max(rim.x, rim.y));
}

@fragment
fn fs(in: Varying) -> @location(0) vec4<f32> {
    // The ink over its rim over the fill, as much of each as its own share
    // says.
    let drawn = max(lines(in.lattice, 1.0) * 0.5, lines(in.lattice, BAY));
    let inked = in.ink.a * drawn.x;
    let rimmed = in.rim.a * drawn.y * (1.0 - inked);
    let filled = in.color.a * (1.0 - inked - rimmed);
    let alpha = inked + rimmed + filled;
    let paint = (pow(in.ink.rgb, vec3<f32>(2.2)) * inked
        + pow(in.rim.rgb, vec3<f32>(2.2)) * rimmed
        + pow(in.color.rgb, vec3<f32>(2.2)) * filled) / max(alpha, 1e-4);
    // The side as it faces the eye, from how the picture changes across it.
    let flat = normalize(cross(dpdx(in.relative), dpdy(in.relative)));
    let normal = flat * -sign(dot(flat, in.relative));
    let shade = 0.8 + 0.2 * dot(normal, surface_up(in.relative));
    let day = sunlight(in.relative) * daylight(surface_up(in.relative));
    let color = paint * shade * mix(0.35, 1.0, day);
    let distance = length(in.relative);
    return vec4<f32>(through_medium(color, in.relative / distance, distance), alpha);
}
