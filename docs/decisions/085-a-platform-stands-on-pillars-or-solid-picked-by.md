# 85. A platform stands on pillars or solid, picked by the button that lays it (decided; the bases are 105's)

Logged 2026-09-30.

A platform's base is pillars, as in 78, or solid: every column under the slab
filled down to the lowest ground at its foot, a block standing on the slope.
The panel lays one or the other with a button each, Platform and Solid, and
`lay_platform` carries the base, pillars when it is left out.

**Why.** Marlus wants the choice by place: pillars read well over some ground,
and a block that meets the hill is the better look in others. The base is
picked as the platform is laid, one click, so it is not a setting kept in the
engine and repeated in `tool_changed`.

**What it weighs.** Nothing in memory: a chunk is stored whole, 4 KiB, and
pillars stand at every bay of 16, one chunk, so they already touch every
chunk a solid base fills. Measured over a plot of 64, on a slope of 5 m and
on the steepest plot near the test world, 32 m of fall across its 32 m:

| | 5 m, pillars | 5 m, solid | 32 m, pillars | 32 m, solid |
|---|---|---|---|---|
| cells | 4,413 | 24,452 | 6,134 | 134,472 |
| chunks stored | 32 | 32 | 61 | 61 |
| quads drawn | 9,236 | 10,884 | 13,598 | 20,806 |
| the frame it is laid in, WASM | 6.8 ms | 7.8 ms | 9.8 ms | 15.6 ms |

The quads are the cost. Inside a solid base no side shows, so what it adds is
its outer walls and the staircase of its underside, which the ground hides
and which is still drawn and still casts. A solid base is one box for each run
of level ground along a row, so a slope is laid in a few boxes a row.

On the steepest plot the frame a solid base is laid in passes the budget of
12 ms, once, at the click. That, and a Pi paying several times as much, is in
OPEN.

**Rejected:** a base kept as a setting beside the side of the platform, a
second control to set before the button that uses it; filling to the ground
as it is, which cubes cannot follow, so the foot of a column is the lowest
ground at its corners and no ground shows between it and the slope; leaving
out the faces the ground hides, which a volume cannot know, since it knows no
ground.

**Lives in:** WORLD.md § the build layer, ARCHITECTURE.md § Clients and UI,
`crates/voxel/src/platform.rs` (`Base`, `Platform::gestures`),
`crates/client/src/build.rs` (`lay_platform`).
