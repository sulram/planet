# BRIEF: nature is a surface, building is volumes

The one current campaign, decided in DECISIONS 58. Replaced
`TERRAIN_RENDER_BRIEF.md` (shipped as M1.5) and `VOXEL_BRIEF.md` (superseded).
Steps 0 and 1 of the order of work are done; the rest is ROADMAP M1.75.

**There is one brief at a time.** When it ships it dissolves: the world into
ARCHITECTURE, the why into DECISIONS, the rest into ROADMAP. Two dead briefs
in every session's context is what this rule is for.

## The decision in one breath

- **The terrain layer is a surface again.** Nature is a heightfield, sampled,
  band limited, beautiful, and not editable by anyone in world.
- **The build layer lives only inside a volume.** A volume is an integer
  address box. Inside it the world is cubic voxels with an inside.
- The two meet at a **containment boundary**, authored and integer, never at a
  LOD boundary. That is the whole difference from the hybrid DECISIONS 48
  killed, which had two representations of the *same* ground handing over
  where a quadtree ran out of levels.
- **The sea stays as it is**, its own surface, and a volume may sit under it.

## What Veloren proves

The full reading is DECISIONS 57. `refs/veloren` is GPL-3.0-or-later, read for
architecture and never copied. Its default `terrain_view_distance` is **10
chunks**: voxels for about 320 m, a heightmap for the rest. A shipped voxel
game chose not to build a voxel pyramid.

- **The far field is textures**, not chunks: height, horizon, colour, sampled
  in the vertex shader. Nothing streams, cracks, pops or needs a budget. We
  already bake this: a `Field` is that, and `ruggedness_m` is the bound
  DECISIONS 56 said the generator lacked.
- **A horizon map** gives terrain self shadowing at any range with no shadow
  map, which is where DECISIONS 55 stopped.
- **A smooth surface can be made to look voxel per pixel**, by a short ray
  march in the fragment shader. The seam of a mixed design goes away there.
- **Light is baked into the mesh**: flood fill sun and per face AO at mesh
  time, on a worker, free per frame. This is what makes an interior read as
  an interior, and what a build volume needs.
- **What does not transfer**: their radial splay, which has nowhere to go on a
  sphere, so our quadtree of patches stays. And nature being editable, which
  is the whole of what we give up.

## The shape

### Nature: the terrain layer

- A heightfield over the quad sphere, as before DECISIONS 48, with what M1.5
  already shipped: water, shore per pixel, grass, clouds, cascades, tone map.
- **Sampled from a baked field, not from the generator per patch.** Alt,
  ruggedness, horizon and colour, baked per sector once at world creation and
  frozen with the recipe. A patch then costs a texture read.
- That bake is also the **bound the generator never had** (DECISIONS 56,
  OPEN): `ruggedness_m` says how far elevation ranges inside a texel.
- Collision keeps reading the field directly. A footing is 2 us today.
- Not editable in world. Shaped at the recipe level, by the landlord, through
  volume stamps below.

### Construction: the build layer

- Cubic voxels, one quad per side that shows (75), **only inside a volume**,
  the unit of streaming, storage, permission and budget. All four already
  wanted to be the same box (M3, M4).
- A volume has an inside: digging, cellars, caves and overhangs live here and
  nowhere else.
- **Proximity streamed**, in the manner of Cryptovoxels: far away a volume is
  a silhouette, not cells.
- Light baked at mesh time: flood fill sun, per face AO, a glow channel.
- `crates/voxel` holds cells, gestures, sight, faces and footing (75).

### The platform: built, and the ground left as it is

- A build stands on a slab of cells on pillars down to the ground, and the
  ground under a volume stays nature's (78, WORLD.md § Two layers).
- **A stamp is how what is not a volume seats into terrain**: a flatten and
  blend footprint applied by `sample_at`, part of the recipe and never an
  edit, at every LOD level by construction. A cave mouth, below.
- One read path survives: the ground is the field plus the stamps over it.

### Water: the sea stays, and volumes go under it

- The sea remains its own surface at its own radius, with what it has today:
  Fresnel, depth colour, refraction, absorption, Snell's window, shadows on
  the surface and on the floor.
- **A volume may sit below sea level**, and none of what that needs is new:
  its cells take the underwater treatment already written, a volume crossing
  the waterline must not punch a hole in the sea, and the sea draws after it.
- No fluid simulation. Water is not a cell type: it is a surface at a radius,
  and cells are simply under it.

## Lighting, stated as a spec

"Perfect" is not a criterion. These are:

- **No sub cell speckle.** It was surface nets vertices wobbling under one
  cell (DECISIONS 55, OPEN). With nature as a surface it cannot occur.
- **No acne on cubes.** Normal offset bias is exact on an axis aligned face.
- **Per vertex AO on every cube face**, from the four neighbours.
- **Baked flood fill sun inside a volume**, so an interior is dark and a
  doorway is a gradient, with no extra shadow pass.
- **One material contract** for the smooth shader and the cube shader
  (DECISIONS 41), so construction is lit by the same sun as the ground.
- **No leak where a volume meets the ground**, and AO continuous across it.
- **Terrain self shadowing at range from the horizon map**, not from a
  cascade, so nothing coarse ever enters a shadow map.

## What this forbids, and the escape hatch

- **Forbidden**: a tunnel in open wilderness, a well in the middle of nowhere,
  a cave the generator made and a body can walk into. Thrown away with it:
  `e43e91a` and `95563ef`.
- **A cave is a shell and a room**: a GLB entity for the rock, seated into the
  ground by a stamp, with a small volume inside where people dig and build.
  Authored, never generated, which is also why it will be worth entering.
- It needs **no new subsystem**. Entities are M5, volumes are M4, stamps are
  here. Caves are therefore deferred rather than designed in, and the line
  above stands until both arrive.
- **The rule underneath**: everything that is not the terrain surface is an
  entity, a volume, or both. Same streaming, storage and permissions. The
  design unifies rather than bifurcates, which answers DECISIONS 48's
  objection rather than dodging it.

## What survives from `feat/voxel`

Do not check out `feat/world-earth`. Branch from `feat/voxel` and restore.

- **Keep whole**: `crates/topology`, with `Grid`, `QuadSphere` and
  `sector_bits` in the recipe (49, 50), property tested at every grain from one
  cell per face to `2^16`; `crates/voxel`, the chunk blob; and the bench
  harness reporting the worst frame and how many are over budget.
- **Deleted, to be written again**: `crates/voxel` and `crates/terrain`. The
  volume streamer wants what level 0 of the latter did (chunks around a body,
  generated, meshed, streamed by distance under a budget), and it will be
  written against a volume rather than a planet. `feat/voxel-wrong-path` is
  the archive.
- **Restore**: `client::terrain`, the heightfield quadtree, by reverting
  `d2cc66c`. It is 19.6 KB and touches `SECTOR_BITS` in two places, so
  reconciling it with the size being a recipe value is small surgery.
- **Lessons, not code**: 51 (a budget counts the work, not the results), 53
  (nothing comes down before its replacement is up, which is M1.5's open
  geomorph item), 55 (nothing coarse casts).
- **Dead weight for nature**: the 3D density path and surface nets. Both stay
  in the tree, because a volume brush will want them.

## Order of work

0. [x] **The docs, first**, because they enter every session. `ARCHITECTURE.md`
   split into it, `WORLD.md` and `RENDER.md`; both dead briefs deleted;
   GLOSSARY, ROADMAP and OPEN brought to 58.
1. [x] Branch, restore the heightfield, delete `crates/voxel` and
   `crates/terrain`, reconcile with `sector_bits` being a recipe value.
2. Fix the generated shape: the wall, the missing shelf, the footprint, the
   unfiltered noise. Four faults, all in `plates.rs`, all already named.
3. Bake the field: alt, ruggedness, horizon, colour, per sector, from the
   recipe. Patches read it. Measure the settle time against 7.4 s.
4. Horizon map in the shader; nothing coarse in a cascade.
5. Per pixel voxelization of the far ground, so the seam stops showing.
6. The volume, taken ahead of 3 to 5 (76). Done: one opens where you stand,
   on a platform, its sides are drawn, and it is built in by click and drag. Left:
   bake its light, stream it by proximity, put one under the sea.
7. Persist a volume and permission its edits. That is M3 and M4 arriving
   together, because they were always the same box.

## What it must not cost

- The frame on a Raspberry Pi, a phone and a browser tab. `bun run bench` is
  the referee and the numbers to beat exist: worst 3.75 ms of 12, median 1.89.
- The far shimmer that 29, 41 and 43 earned.

## What this brief does not decide

Those live in OPEN.md and nowhere else: the block edge (0.5 m or finer), the
chunk side (16 or 32), the build band height, whether a volume may be placed
anywhere or only where the landlord opened the map, and what a volume costs a
person in quota.

**And the one the cave forces: what a body collides with inside a GLB shell.**
DECISIONS 47 says collision is the smooth field, always. A shell is a mesh, and
we have no mesh collision. Either the volume is the truth and the GLB is a skin
over the same cells, which keeps one collision rule and costs a voxelizer at
import, or mesh collision becomes a capability we build. The first is cheaper
and keeps the invariant; neither is decided here.
