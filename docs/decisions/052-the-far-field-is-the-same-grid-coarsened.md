# 52. The far field is the same grid, coarsened (decided)

Logged 2026-09-21.

Pull the camera back and the world ended: there was one level, it reached 56 m,
and past that there was nothing. A body has to be drawable from any distance -
the moon is seen from the Earth - and the ground under the avatar's feet most
of all.

**A level is a grain, not a different thing.** A chunk of level `L` holds cells
`2^L` blocks wide. Its cells live on the block grid coarsened by `L`, and the
chunks on that grid coarsened again by `CHUNK_BITS`. Everything that resolves a
seam, a neighbour or a direction already took a `Grid` and did not care how
coarse it was (49), so the pyramid inherited all of it, and the property tests
that run at every grain from one cell per face upward were already covering the
levels before any existed.

**A coarse chunk is generated, not built.** The brief left both open: build a
reduced chunk from the eight under it, or ask the generator at a coarse
footprint. The generator already takes `footprint_m` and fades out what a mesh
cannot carry (29, 43), so asking it is both cheaper and the band limited
answer. Building from below becomes necessary when chunks are stored and edited,
because then the fine chunks hold something the recipe does not; that is the
same problem as one read path per level and is not solved here.

**A level wants a shell.** Close enough that the level is worth drawing, far
enough that the level under it already covers the ground. `DETAIL` is the only
knob, and it is honest about its cost: every level is a square
`2 * DETAIL + 1` chunks across, so raising it raises the work at every level at
once. The coarsest level has no outer edge, which is what makes a body
undisappearable.

**A level is reworked only when the eye leaves its own chunk.** At level 0 that
is every 8 m; at level 12 it is every 32 km. Without this the scan would be 13
levels deep every time the avatar took a step.

Measured at `2^10`: 1,103 chunks drawn and the whole planet visible from 900 m
up in 73 of them. `bun run bench`: worst frame of a descent 5.68 ms of a 12 ms
budget, median 2.16, none over. Settling a world from nothing at `2^16` is
7.4 s of work spread over frames, and holds 16,642 chunks.

Also fixed here, and it was a plain fault rather than a missing feature:
streaming was centred on the camera, so pulling the boom back unloaded the
ground under the avatar. It is centred on the body now; the camera only looks.

Rejected: raising the reach of a single level, which is the window 48 is about
wearing a larger coat; building reduced chunks from the level below, which is
right once chunks are stored and wrong while they are not, because it costs the
whole pyramid underneath to draw the top of it; skirts to hide level
boundaries, not because they are beneath us but because nothing has yet shown a
boundary that needs hiding, and a fix with no fault to point at is a constant
waiting to be tuned.
