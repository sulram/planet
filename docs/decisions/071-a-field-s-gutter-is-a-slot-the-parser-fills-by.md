# 71. A field's gutter is a slot the parser fills by folding the seam in address space (decided)

Logged 2026-09-27.

`Field::parse` fills each face's one texel gutter from the neighbouring face's
interior, folded across the seam with the grid's own fold (`Grid::wrapped`);
where three faces meet, the corner texel is the nearest integer to the mean of
the three corner texels. The bake leaves the slot empty. The sampler stays
what it was: bilinear over ordinary texels, no seam logic and no branch per
sample.

The Earth world at `4-NZZTM3R@207,71,-5` showed stacked ribbons and sky
through the ground along the sector 4/0 seam. The bake filled the gutter by
projecting each gutter texel's direction onto the neighbouring face, and the
projection shifts the coordinate along the edge, so each face interpolated a
boundary of its own and the two heights disagreed along the whole seam; a
vertex on the seam then fell to either side by floating point luck, and the
mesh alternated between them. In address space the texel past the edge is the
neighbour's first texel, exactly as a step across a sector edge lands on the
neighbour's first block; both faces then interpolate the same pair of texels,
and the shared corner tap makes the three limits at a cube corner agree.

The fold is topology's, so it is written once, in Rust, and runs at load: four
edges of texels per face per level, in place, no allocation. An existing
PLFIELD1 file is valid as it is, since whatever the bake put in the slot is
overwritten; the next bake writes zeros there and gets a new content id.

**Rejected:** folding per sample in the sampler, which puts a branch on every
tap and a seam fold on every edge sample, on the hottest path in WASM, to
compute at every frame what is known at load. Folding in the bake, which
writes the seam adjacency a second time, in TypeScript, and lets the bake and
the reading drift apart the way the first bake came out upside down. A
gutter-free file expanded at load, which copies 25 MB in a browser tab to save
a hundred kilobytes of slot. Deeper skirts or two-sided rendering, which hide
some holes and keep two heights. Snapping mesh vertices, which leaves
collision discontinuous.

**Lives in:** WORLD.md, `crates/worldgen/src/field.rs`, `scripts/field.ts`.
