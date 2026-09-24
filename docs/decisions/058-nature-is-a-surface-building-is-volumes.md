# 58. Nature is a surface, building is volumes (decided)

Logged 2026-09-21.

This retires 48. The world is not voxels to the core: the terrain layer is a
heightfield again, not editable by anyone in world, and cubic voxels exist only
inside a volume, which is an integer address box where building is granted.

**Why 48 was right to kill the hybrid and wrong about what to build.** Its
diagnosis holds: a heightfield with a voxel window bolted to the deepest
quadtree level is two representations of the *same* ground handing over where a
quadtree runs out of levels, and that seam let the sky through. Its remedy,
voxels at every level to orbit, was measured over four weeks and priced: a world
takes 7.4 s to settle and holds 16,642 chunks at `2^16`, of which 46% say one
thing all through and were generated anyway (56); the sea, the grass and the
moon were lost; and a sub cell speckle on lit slopes survived every shadow fix
(55). None of that is payable on a Raspberry Pi, a phone and a browser tab.

**What changes is the kind of seam, not the number of representations.** A
volume meets the terrain at a **containment** boundary: authored, integer, and
decided by a person or by the recipe, never by how far the camera is. Nothing
hands a solid over to a surface because nothing is both.

**Veloren settled it** (57): a shipped voxel game draws real voxels for about
320 m and a heightmap past that, its far field is three textures rather than a
chunk pyramid, and its light is flood filled into the mesh on a worker. Our
`Field` already is that far field, and `ruggedness_m` is the bound on the ground
inside a texel that 56 said the generator did not have.

**What this forbids, and it has to be said plainly.** No tunnel in open
wilderness, no well in the middle of nowhere, no cave the generator made that a
body can walk into. The cave work of 47 and the density path that fed it are
kept in `worldgen` because a volume brush will want them, and `client::collision`
stops walking a density column: a footing is the ground height, which is exactly
what the terrain mesh is built from, so the two can no longer disagree.

**A cave, when there is one, is a shell and a room**: a GLB entity for the rock,
seated into the ground by a stamp, with a small volume inside where people dig.
Authored, never generated. It needs no new subsystem, so caves are deferred
rather than designed in. What a body collides with inside that shell is open.

**A stamp is how anything that is not terrain seats into terrain.** A volume
carries a flatten and blend footprint applied when the ground is sampled, so a
platform is part of the recipe and not an edit. Deterministic, small, server
side, and applied at every level by construction. One read path survives.

Deleted here: `crates/voxel` and `crates/terrain`, which will be written again
from scratch when volumes are built, and `client::tests::caves`, whose feature
is gone from nature. `feat/voxel-wrong-path` on the remote is the archive; a
graveyard directory in the workspace would only keep compiling and keep
entering context.

Rejected: keeping the pyramid and raising the budget, which is how a design
stops being measured; a deprecated crates directory instead of deleting, which
is a museum that costs a build; caves as generator placed volumes from the
start, which designs in a subsystem nothing yet needs.
