# WORLD

What a world is made of: its grid, its layers, its recipe, how it
streams, and which way is down. The core, its plugins and the server are in
ARCHITECTURE.md; how the picture is made is in RENDER.md.

## Topology: quad sphere, single build band

- Six sectors (cube faces), each a square grid, projected onto the sphere with
  the tangent warp (25): grid coordinate `s` moves to `tan(s * pi / 4)` on the
  face. A block edge is 0.5 m at a sector centre, 0.35 m at the corners.
- Seams are computed, not tabled: a column is an integer point on a cube, a
  step over an edge is one vector sum, swaps and flips fall out of the frames.
- Address: `sector (0..5), u, v, h` then chunk index and block index.
- Two levels: a `Grid` is six square faces at one grain and owns seams,
  neighbours and the warp; a `QuadSphere` is one body, a block grid plus the
  radius that turns cells into metres. The chunk grid is the block grid
  coarsened by `CHUNK_BITS`, so chunk seams are block seams (49).
- Block edge 0.5 m (75). Blocks per sector side is `2^sector_bits`, a recipe
  field frozen per world, `4..=16`: 16 because u and v fill a `u16`, 4 because
  a sector must hold a chunk. Radius follows, four sector sides to a great
  circle: 5.09 m at `2^4`, 326 m at `2^10`, 20.9 km at `2^16`.
- Build band: `+-min(256 blocks, radius / 4)` around **the surface**, not the
  datum, bedrock at its floor. The 256 is a human measure, a cellar and a
  tower, and is the same on any body; the quarter of the radius is the hollow
  core keeping its share. At `2^16` that is the +-128 m it has always been (p),
  and a column tapers 0.6%, so the band is a regular grid in practice. On a
  small body the radius wins and the taper is the price shells will pay (49).
- A sector's edge plots, its 8 corners with them, are nature: no volume (77).
- The moon is a quad sphere of its own, `2^14` blocks a side, 5.2 km of radius:
  a volume stands on it as on the planet, cut by its grid, from its centre.
- On foot the planet reads as flat: horizon at about 260 m from eye height.
  The curve shows from altitude.

## Saying where you are

- The address is integers; a **place code** is those integers spelled so a
  person can read them out, write them down and paste them. It is the address,
  not a name beside it, so the two can never disagree.
- `(sector, u, v)` with the bits of `u` and `v` woven together, most
  significant first, `u` leading, spelled in **Crockford base32** (no `I`, `L`,
  `O` or `U`, the ones misread on paper and misheard out loud). Reading one
  back ignores case and the dash, because neither survives being copied.
- **A prefix is a box**: codes that start alike are near each other, and
  cutting characters widens the box around the same spot. **Length is
  precision**: three characters name about 128 by 256 m of the reference body,
  seven name one block. An odd length leaves the box twice as long as it is
  wide, because a character is five bits and the stream alternates.
- Written against the reference body, so the same code names the same fraction
  of a sector whatever `sector_bits` a world has.
- A **pose** is what a link carries. A place shows *where you are*; a pose puts
  someone *where you stood, looking at what you looked at*, which is how a
  gallery gets shared:

```text
m4-K7M42Q@40,180,-5
│└──┬───┘ └┬┘ └┬┘ └┬┘
│   │      │   │   └── pitch, degrees, positive looks up
│   │      │   └────── bearing, degrees clockwise from north
│   │      └────────── height in blocks from the datum, when not on the ground
│   └───────────────── the place code
└───────────────────── the body: nothing is the planet, `m` is the moon
```

- Everything is optional from the right but the code, so the common case stays
  short: outdoors, `4-K7M42Q` is the whole of it. Leaving the height out is
  what standing on the ground *means*, and it is why a shared link survives:
  the ground is a function of the recipe.
- **Two renderings, not two facts.** A HUD shows the place, which is what a
  person reads out; the address bar carries the pose. `--at` takes the pose, so
  what is in the address bar pastes straight into a headless render, and a
  pose pasted over the address bar is walked to (73).
- North is the `+Y` pole. Not a choice: the sun turns about `+Y`, so it is the
  axis that gives a world its time zones, and a compass has to agree with the
  sky. At a pole there is no bearing, and that is said rather than guessed.
- The compass reads where the **camera** looks, not where the body points:
  turning the mouse leaves the body where it was, and "which way am I looking"
  is a question about the eyes.
- The rose is not in Rust. N, S, E and W are English and the Portuguese rose
  runs N, NNE, NE, ENE, **L**, so the engine sends the angle and the front end
  names it.

## Two layers: a surface, and volumes

| Layer | What it is | Mesher | Edited with | Where |
|---|---|---|---|---|
| Terrain | one ground per direction, sampled | quadtree patches | nothing, in world | everywhere |
| Build | air or a paint per cell, an inside | one quad per side facing air, each corner bent onto the body | create, delete, paint | inside a volume |

- Nature is a surface (58): one ground per direction, no cave, no overhang,
  nothing to be inside of, and not editable by anyone in world. What is built
  leaves it as it is (78).
- A volume is an integer address box with an inside: cellars and overhangs.
  The world keeps it, and tells a change to whoever is near (107, 110).
- The address cuts volumes (77): plots of 64 blocks a side, `(u >> 6, v >> 6)`,
  a volume over each from its lowest ground to 1024 cells over its highest.
  They touch and are read as one, so a build stands over two neighbours.
- A build stands on a platform (78): a slab of cells, 8 to 64 a side as
  picked, cut by the address, its top the higher of the ground under it and
  the feet, as a deck, solid or floating, laid where it is asked for (105).
- The two meet at a containment boundary: authored, integer, and decided by a
  person or by the recipe, never by where a quadtree runs out of levels.
- A stamp seats what is not a volume into terrain: a flatten and blend
  footprint read when the ground is sampled, in the recipe, never an edit.
- A cell is a 0.5 m block of the address, air or a paint; a gesture fills air,
  empties or repaints over a box (75). Chunks of 16 a side are what is stored
  and redrawn, and an empty one takes no room. All in `crates/voxel`: no sphere.

## The world is a recipe

- World = seed + params + generator version. Founding a world writes it once.
- Wire shape, shared unmapped by Rust, Go, TypeScript and the world
  folder: `{seed, generator_version, params}`. The seed is a u64 written
  as 16 lowercase hex digits (JSON numbers stop at 2^53).
- A founded world's recipe is frozen: the server refuses a second founding.
- Generator v3 (new worlds): the shape is a source, and the body below it is
  v2's. `params.source` is `generated` (tectonic plates over the seed) or
  `{field}` (a baked cube map of a real body, named by content id).
- Generator v2: eroded massifs, a sea floor, and
  `sample_at(direction, footprint_m)` which fades detail finer than the mesh
  that asks. Collision and saves use full detail. v1 stays frozen beside it.
- Generator v1: continents, ridged mountains and detail as 3D simplex noise;
  materials water, sand, grass, forest, rock, snow. Params: `relief_m`,
  `ocean_depth_m`, `continent_scale`, `sea_share`.
- The build plugin's store holds only modified chunks and the op log.
- Read path: stored chunk if present, else generate. One function, everywhere.
- First edit to a chunk: generate it, apply the edit, store the whole chunk.
- Op log: who, when, address, before, after: undo, audit, the snapshot hash.
  A volume is the unit kept, permitted and rolled back to a moment (95).
- The generator is written once in Rust: native in clients, WASM in the
  browser, the same WASM inside Go through wazero (pure Go, no CGO).
- 3D noise sampled on the sphere: no seams, no projection distortion.
- `density_m(direction, height_m, footprint)` is the volume's reading of the
  same ground: it crosses zero at exactly the height `sample_at` reports, so
  a volume chunk and a heightfield patch have nothing to reconcile where they
  meet. A cave lives only here, because a height has no room for one; v1 and
  v2 are frozen solid all the way down.
- `column(direction, footprint)` works out everything that does not change
  with height, and a column with no cave in it answers from two numbers. A
  volume walks one line asking for tens of samples: paid per cell, a patch of
  65,536 cells costs 40 ms in WASM, and paid per column it costs 4.94.
- A cave is taken out of the ground, not subtracted from it: the density is
  the nearer of the rock above and the nearest tunnel wall. Subtracting a
  carving depth would make a cave something that must beat the weight of rock
  over it, so caves would only ever open a few metres down. Two ridged sums
  crest along surfaces and meet in a line, and a line is a passage; a region
  field decides where cave country is, so how common a cave is and how wide it
  is are two knobs and not one. Caves stop at the sea and at the floor of the
  build band.
- A **field** is a cube map of ground, one face per sector, with a mip pyramid
  and a one texel gutter across each seam. Two channels: elevation (`i16`,
  metres on the source body) and ruggedness (a byte of 16 m steps, the spread
  inside a finest texel). Levels blend by footprint, as `band` fades an octave.
- The gutter is a slot: the bake leaves it empty and `Field::parse` fills it
  from the neighbouring face, folded across the seam in address space, with
  the mean of three faces at a cube corner (71). Sampling then knows no seam.
- A field gives shape, never height: at 1/305 of Earth, honest elevations are
  a billiard ball and honest exaggeration is a wall. Zero maps to zero, so the
  coastline is exact; the relief is the generator's, sized by `relief_m`.
- Fields are baked by `bun run field` into `assets/fields/` (gitignored), each
  with a sidecar naming its content id. Default: ETOPO 2022, public domain,
  1024 texels per face side (32 m of planet, 9.8 km of Earth), 25 MB.
- Params are one table in `$lib/world.ts`: range, step, default and the shapes
  each means anything for. They are sliders on the founding screen and parameters
  of its address, clamped there; the server checks a recipe's shape alone. A knob at its
  default is absent from both.
- The sea takes an exaggeration of its own (`sea_curve`), because a depth in
  proportion to a real body leaves every strait a shoal, and `sea_level_m`
  moves a field's coastline the way `sea_share` moves a generated one.
- A recipe that names a field cannot be generated without it: the shell reads
  it (`--field` on desktop, one fetch on the web) and hands it over before the
  recipe. `Generator::new` refuses; `Generator::with_field` checks the id.

## Streaming and LOD

- The terrain is a quadtree of patches per sector, ground to orbit, per body.
  Each node is a square of address space meshed as one patch of `PATCH_GRID^2`
  quads; near the camera nodes split, so triangles stay about the same size on
  screen from the ground to orbit.
- A patch samples the generator at its own footprint, so a coarse patch is a
  band limited version of the fine one and a hill does not change size because
  the camera moved (29, 43).
- A node keeps drawing itself until its children are ready: never a hole. Six
  patches are built an update, closest first, one of them for its grass (86).
- Streaming is around the body, never around the camera: the camera is a boom
  that swings metres away and may look from orbit, and the ground under the
  avatar may not depend on where it points.
- One read path inside a volume: `chunk(addr)` is the stored chunk if there is
  one and the generated chunk otherwise. On arrival the client asks which
  chunks near it are stored, at what version, and the server sends only those.
  Untouched nature costs zero bandwidth.
- Client cache: SQLite on native, OPFS in the browser.

## Gravity

- Gravity is a field, decoupled from geometry. Shapes: sphere, box, parallel,
  more later. Range + priority; the highest-priority field containing you wins.
- Constant strength, direction only. The avatar's up vector eases to the
  opposite of gravity. Input is projected on the plane normal to gravity.
- The planet is one sphere field. Fields are entities: streamed, permissioned.
