# 44. A world's shape is a source: plates, or a baked field (decided)

Logged 2026-09-21.

A world could only ever be its seed. People want to walk the Earth, and the
answer is not a second generator: it is that the body of the generator should
not know where its shape came from. Generator v3 reads two broad numbers,
`land` and `ranges`, plus a `lift`, and everything under them, the coast
easing, the shelf and abyssal profile, the eroded massifs, the materials, is
the tuning v2 already earned. `params.source` picks who answers. `generated`
lays about eighteen tectonic plates over the sphere, each with a Euler pole,
and reads the boundary between the two nearest at every sample: convergence
raises a cordillera or digs a trench with an island arc beside it, divergence
opens an oceanic ridge or a continental rift. That is why v2's ranges came out
as patches and these come out as arcs; a range is the edge of a plate, not a
blob of noise. `field` reads a cube map baked from a real body, one face per
sector, and the seed still owns everything below a texel, so one field is a
family of worlds that share a coastline rather than one world.

The field is not used as a height, and this is the whole of the scale problem.
Our planet is 1/305 of Earth: honest elevations make a billiard ball (Everest
becomes 29 m), and the exaggeration that fixes that, about 48x, turns every 1
degree slope into 48 degrees. What survives the change of scale is the shape.
Zero maps to zero, so the coastline lands exactly where the source has one,
and the relief inside a belt is the generator's own. The first attempt fed
elevation and ruggedness straight in at texel resolution and grew a forest of
needles: the broad channels are now read at their own footprints (250 m for
lift, 1000 m for ranges) while the coast stays at the mesh's.

Two channels per texel: elevation as `i16`, and ruggedness, the spread of
elevation inside a finest texel, as a byte of 16 m steps. Ruggedness is what
tells a cordillera from a plateau of the same height, and coarse levels average
it rather than accumulate it, so a silhouette cannot change because the camera
moved away. Levels blend by footprint exactly as `band` fades an octave. Each
face carries a one texel gutter filled from the ground that continues past the
edge, so a seam is an ordinary texel and no seam logic runs per sample. In
WASM the field path costs 0.90 ms per patch against a budget of 1.5, and its
descent is cheaper than the generated one (6.99 ms against 9.81): fetches beat
octaves.

The default field is ETOPO 2022 (NOAA NCEI), a work of the United States
government and so public domain, baked by `bun run field` to 1024 texels per
face side, 32 m of planet and 9.8 km of Earth per texel, 25 MB with its mip
pyramid. A recipe names a field by the content id the bake writes into its
header, and the generator refuses ground that is not the ground the recipe
names: a world shaped by other ground is a different world, and its stored
chunks would no longer line up. `Generator::new` refuses a recipe that needs a
field, so nothing can quietly fall back to another planet.

Rejected: an API call at world creation (the endpoint moves, the grid is
revised, and the same recipe silently generates another planet under chunks
already saved; a field is downloaded once, baked once and frozen by its hash);
equirectangular storage (a seam, a pole singularity, and trigonometry per
sample, against a cube map that is already our sectors); a hydraulic erosion
bake for the generated source (a week of work and a `Generator::new` that is
no longer instant, and plates alone carry most of the reading; it stays a
ROADMAP wish); shipping the field in the repo (25 MB, and `bun run field`
rebuilds it, so it is gitignored like the rest of the bake).
