# 86. A build is drawn over frames, and no frame pays for one whole (decided)

Logged 2026-09-30.

A change to cells owes a mesh to each chunk it touched, and the chunks are
meshed eight an update, the nearest the eye first, each keeping the mesh it
had until its new one is made. The ground under a platform is read thirteen
rows an update and the platform lands when all of it is read, in an update
that meshes nothing. Grass that gives way under a build is built again a
patch an update, out of the terrain's budget of six, first. A chunk's sides
are read from a copy of it and one cell all round, a row of a chunk at a
time, by fixed steps.

**Why.** Marlus worried over the frame a platform is laid in, first for its
own sake and then for what happens when every build of everyone nearby
reaches every screen. A platform of 64 on a solid base over the steepest plot
near the test world cost 29.5 ms in one frame in WASM against a budget of 12:
21.7 ms for the command, which read 4,225 samples of the ground, laid the
cells and meshed 61 chunks, and 7.8 ms for the next frame, which built six
patches of terrain again for their grass. Each part is spread over frames
now, and `bun run bench` holds the worst of them under the frame budget: the
command 0.19 ms, the worst frame after it 8.68 ms, the platform drawn whole
in about ten frames and its grass gone in 39. Those numbers were read beside
a browser running the world, which puts the other cases 10 to 20% over their
usual.

What arrives from a server will take the same road: chunks that change or
come into reach are owed a mesh and drawn in turn, so a place full of builds
is walked into over frames and never lands in one.

**The sides of a chunk.** Read through the plots cell by cell, a solid chunk
asked for tens of thousands of cells, each through a lookup. The copy is
read a chunk row at a time and a side's neighbours are fixed steps away in
it. It halves nothing a frame shows: the frames were the terrain's. It is
kept because it is the shape a chunk from a server arrives in.

**Rejected:** a budget in milliseconds, which WASM cannot read without a
clock of the host's and which makes a headless picture depend on the
machine; meshing on a worker, which the web client has no threads for yet;
keeping every tuft of every patch to filter again, megabytes where a Pi has
little, where a patch built again costs one of the six builds of an update;
reading fewer samples of the ground and interpolating, which lets ground show
between a column and the slope.

**Lives in:** RENDER.md § Volumes, WORLD.md § Streaming and LOD,
`crates/client/src/build.rs` (`MESHES_PER_UPDATE`, `GROUND_ROWS_PER_UPDATE`),
`crates/client/src/terrain.rs` (`REGROWS_PER_UPDATE`),
`crates/voxel/src/faces.rs` (`Padded`), `scripts/bench.ts`.
