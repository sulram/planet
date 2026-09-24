# 57. What Veloren has to teach us, and what it does not (noted)

Logged 2026-09-21.

`refs/veloren` is a Rust voxel game people play, GPL-3.0-or-later, read for
architecture and never copied. It is the counter-example to 48, and it is worth
saying plainly what it chose: its default `terrain_view_distance` is **10
chunks**. Real voxels for about 320 m, and a heightmap for everything else. A
shipped game with a full voxel world decided not to build a voxel pyramid, and
56 already suspected this was the reading that would pay.

**The far field is textures, not chunks.** Three of them: `t_alt`, a 16 bit
height; `t_horizon`; `t_map`, a colour. One mesh, fixed, sampled in the vertex
shader (`lod-terrain-vert.glsl`, `include/lod.glsl`). Nothing streams, nothing
is a quadtree node, nothing cracks, nothing pops, nothing needs a frame budget.
The whole far world is a few megabytes of texture.

We already bake exactly this. A `Field` is a cube map of a body's ground per
sector (44), used today as generator input. It is also the far field, and
`ruggedness_m` is the bound on the ground inside a texel that 56 said the
generator did not have. Baking a generated world's field at creation gives, in
one move, the far field, the bound, and the end of the 7.4 s settle and the 46%
of chunks generated to discover they were empty.

**The horizon map is the answer to 55.** Per texel, the angle of the horizon in
two directions; `horizon_at2` reads it. Large scale terrain self shadowing at
any distance with no shadow map at all. 55 had to stop coarse ground from
casting because one of its triangles spans hundreds of shadow texels and fails
its own depth test. It stopped there. This is where it continues: coarse ground
does not cast into a cascade, it carries its own horizon.

**A smooth surface can be made to look voxel per pixel.** `lod_voxels` marches
a short ray against an implicit grid whose cell size grows with distance, and
returns a cubic normal and an AO term. The far heightfield reads as the same
world as the near cubes without being the same data. That is the seam of a
mixed design made invisible in the fragment shader rather than in the mesh,
which is the opposite of where we spent this month.

**Light is baked into the mesh, not computed per frame.** Sunlight floods
through air at mesh time losing a step per cell (`SUNLIGHT` is 24 in
`mesh/terrain.rs`), and is packed into the vertex beside a glow channel, five
bits each. Per face ambient occlusion comes from the four neighbours of a face
corner. Both are chunk local, both run on a worker, both cost the frame
nothing, and together they are why an interior in that game reads as an
interior. This is what a build volume needs and what no amount of cascade
tuning gives.

**Small things worth stealing.** Indices sorted by altitude, so one mesh is
drawn as the range the eye's depth selects, which is free culling for a cave.
`pull_down`, which sinks the far sheet where the near field takes over so the
two never fight for the depth buffer: blunt, and it works. Instanced LOD
objects for trees and houses, which is what a distant volume should be.

**What does not transfer.** The splay: their far mesh is a fixed grid stretched
radially around the eye, which works in a flat world with a finite map and has
nowhere to go on a sphere seen from orbit. So our quadtree of patches stays.
What changes is that a patch reads a texture instead of calling the generator,
and that is where the cost was. Also not transferable, and the whole of what we
give up: their world is voxels throughout, so digging anywhere is free.

The strategy this points at is docs/BRIEF.md. What it decides goes in its own
entry when it is applied, not here.
