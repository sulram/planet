# 56. What Dust has to teach us, and what it does not (noted)

Logged 2026-09-21.

`refs/dust` is a Rust voxel engine that ray traces a VDB style tree with
hardware ray tracing. Its requirements rule it out for us: it needs
`VK_KHR_ray_tracing_pipeline` and `VK_KHR_acceleration_structure`, Rust
nightly, and Bevy, which 01 and the M1 discipline both refuse. A Raspberry Pi,
a phone and a browser tab have none of that.

**The part that does not transfer, and is worth naming anyway.** Dust does not
mesh. Every problem this week has been about turning voxels into triangles:
cracks between levels, borders a mesh has to read, two surfaces fighting for a
depth buffer, shadow acne from sub-cell geometry. A renderer that marches a ray
down a hierarchy has none of them. That is a real fork in the road and we are
not taking it, because our targets cannot.

**The part that does.** Its internal node holds, per child slot, either a
pointer to a child or a single value standing for that whole subtree, with a
bitmask saying which (`crates/vdb/src/node/internal.rs`). That is our
`Chunk::Uniform`, except VDB has it at every level of the tree rather than only
at the chunk.

Measured against that idea, seed 1: of the chunks held around a body, 20% say
one thing at `2^8`, 32% at `2^10` and **46% at `2^16`** - 7,785 chunks of
16,924. Every one of them was generated, 256 generator columns each, to find
out it was empty. That is roughly two million column samples spent on nothing
and a large share of the 7.5 s a world takes to settle.

We cannot take the shortcut yet, and the reason is worth writing down: deciding
a chunk is uniform without generating it needs a **bound** on the ground inside
its footprint, and the generator has no such thing for a generated world. A
field carries `ruggedness_m`, which is exactly that bound for a baked one. A
generator that answered "the ground here is between these two heights" would
collapse half the pyramid into values on a parent, and would also be what makes
a far level honestly hollow (48).

That is the lesson, and it belongs to `worldgen` rather than to the renderer.
OPEN.md carries it.
