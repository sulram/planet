# Voxel planet implementation brief

The proof of concept that replaces the heightfield terrain with voxels to the
core. Proposed, not committed: the commitment lives in ROADMAP.md and the
decision in DECISIONS.md (48).

## Objective

- Every body in a world is voxels: the planet, the moon, a satellite someone
  builds. One representation, from a block under your feet to a planet seen
  from orbit.
- Digging a tunnel or a well is removing cells, and nothing about the world
  has to be widened, windowed or special cased for it to work.
- Orbit is a requirement, not a later concern: a body is drawn whole from
  outside and in blocks from the ground, with the same data.
- Smaller worlds are a generator option, so the design can be proved at a size
  where every level fits before it is asked to hold 20.9 km.

## Why the heightfield hybrid failed

- A heightfield is a surface of no thickness. It has no inside, so a world
  built on one cannot be entered, dug or looked at from underneath.
- The volume layer (`client::volume`) was a 16 m window of density meshed at
  the deepest quadtree level. Inside the window the world is solid; one metre
  below it there is not rock, there is nothing.
- Every fix was the window again: deeper, then a ring of patches around the
  body, then a cap on how many chunks a patch may mesh, each one a constant
  chosen against a bench. Parked on `feat/voxel-volume` rather than continued.
- What looks like a backface culling fault is the absence of an interior. In a
  voxel world it cannot happen: a face exists only between solid and air, and
  a closed world has no way to be seen from the wrong side.

## The reference

- `https://bowerbyte.com/posts/blocky-planet/`, Bowerbyte, August 2025. A
  spherical Minecraft in Unity: quad sphere, shells, block addresses,
  neighbours across sector seams, radial gravity, 3D noise on the sphere.
- Where we already agree, and by our own route: the quad sphere with a
  pre-distorted mapping (tangent warp, DECISIONS 25), the address
  `sector, u, v, h` with neighbours resolved across seams and corners
  (`topology`, property tested), radial gravity that weakens between bodies,
  3D noise sampled on the sphere so there are no seams and no projection
  distortion.
- Where we have to go further, because that demo does not need to: the planet
  is 20.9 km and not a few hundred metres, so LOD is unavoidable; and the
  world is authoritative on a server by address, so what a client draws may
  never decide what the world is.
- Where it has something we do not: shells, which keep a block's width roughly
  constant as you go down by quadrupling the blocks per layer at fixed steps.
  Our GLOSSARY already names the term; nothing implements it.

## What survives

- `topology`: address, seams, corners, tangent warp, property tests. This is
  the part the article spends half its length on and it is already ours.
- `voxel`: the chunk blob (uniform chunk in 4 bytes, palette and bit-packed
  indices otherwise) and surface nets with a shared low border.
- `worldgen`: deterministic density with caves, `column` hoisting, golden
  hashes, the band limiting discipline (29, 43).
- `render`, `avatar`, `scene`, `client`'s command and event seam, `ui-native`,
  the web app and the server. None of them care whether the ground is a height
  or a volume.

## What goes

- `client::terrain`: the quadtree of heightfield patches with skirts.
- `client::volume`: the window bolted to its deepest level.
- Every rule that exists because the ground was a height: the shown ground a
  flyer is held over, the shore painted by height per pixel, the sea sharing
  the terrain mesh, the grass anchored to patch origins. Each has a voxel
  answer; none of them carries over unchanged.

## The numbers that shape it

A sector side of `2^n` blocks at 0.5 m. Surface chunks are `16 x 16` columns,
so there are `6 * (2^n / 16)^2` of them over a whole body.

| `n` | radius | surface chunks | reduction levels to 1,500 chunks |
|---|---|---|---|
| 10 | 326 m | 24,576 | 2 |
| 11 | 652 m | 98,304 | 3 |
| 12 | 1.3 km | 393,216 | 4 |
| 14 | 5.2 km | 6,291,456 | 6 |
| 16 | 20.9 km | 100,663,296 | 8 |

- Orbit at full voxel is therefore a pyramid of reduced chunks 8 to 9 levels
  deep for today's planet, where one cell of the coarsest level is 128 m.
  That is the same depth the heightfield quadtree already runs at, which is
  the encouraging half of the arithmetic.
- The expensive half: a reduced level is built, not sampled. A heightfield
  patch asks the generator for a coarse height; a reduced chunk has to be
  built from the finer chunks under it, or generated at a coarse footprint,
  and it has to keep the mean or the ground grows and shrinks as the camera
  moves (43).

## The design

### The near field has an inside; the far field may be a shell

- The rule the hybrid got wrong. A far level may be hollow, because nobody can
  be inside it: at 128 m a cell, a body is many levels away from that mesh.
- The near field must be a true volume as deep as anybody can go, which is the
  build band and not a window chosen against a frame budget.
- Every level boundary is then volume to volume. Nothing hands over from a
  solid to a surface, which is the seam that let the sky through.

### Address

- `sector, shell, chunk, block`, the article's sequence, and ours plus the
  shell. `topology::Address` is `sector, u, v, h` today with one implied
  shell.
- Shells matter when digging goes deep enough that a block's width halves.
  They can wait, and the term stays reserved so nothing else takes it.

### One density, two meshers

- Surface nets for nature, greedy cubes for the build layer and for the
  cosmetic blocky view, over the same cells. This was already the plan
  (ROADMAP, M1) and it is unaffected.
- Collision stays the smooth isosurface whatever is drawn (DECISIONS 47), and
  `client::collision` reads a column of density, so it survives as is.

### Bodies

- A body is a voxel world: a topology, a size in `SECTOR_BITS`, a generator
  recipe. The planet, the moon and anything built in orbit are the same type
  with different numbers.
- The moon stops being a second sampling path in the generator and becomes
  another body with its own recipe.

### World size is a recipe parameter

- `SECTOR_BITS` is a compile time constant today, so a world cannot be small.
  It becomes part of the recipe, frozen per world like the generator version.
- This is the first real piece of work and it touches `topology::Address`,
  the generator, the client and the wire shape. It is also what makes the POC
  provable: at `2^10` every level fits and the design can be judged before it
  is asked to hold a planet.

## Order of work

1. `SECTOR_BITS` per world, through `topology`, `worldgen` and the recipe.
2. A new crate beside the old one, so `client::terrain` keeps compiling and
   the working planet is never at risk while the POC is tried.
3. Chunks around a body: generate, mesh, stream by distance, with the build
   band whole so a tunnel and a well work by construction.
4. Reduced chunks and the level pyramid, up to orbit, band limited so a hill
   does not change size with the camera.
5. Shells, when depth distortion earns them.

## What it must not cost

- The far shimmer that the generator and the water already earned (29, 41,
  43). A volume has no footprint of its own, so the filtering has to come back
  in the reduced levels.
- The frame on a Raspberry Pi, a phone and a browser tab. `bun run bench` is
  the referee, and the budgets move only on purpose with the why in DECISIONS.
- The invariants: the address is integer, the server is authoritative by
  address, one read path (`chunk(addr)` is the stored chunk else the generated
  one), copy on first write.

## What this brief does not decide

Those questions live in OPEN.md and nowhere else: the size a world is proved
at and the size the first real world ships at, how far orbit has to reach
before a level may be a shell, when shells arrive, and whether blocky or
smooth is what a visitor sees first.
