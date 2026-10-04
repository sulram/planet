# ROADMAP

Intent, not contract. What is built lives in the theme docs and in git; this
is what comes next. Add wishes freely, reorder as priorities shift, and strike
what we drop with the DECISIONS number that says why. Every step ends runnable
end to end.

## Now: a world mundos hosts

- The campaign and its order of work: [BRIEF.md](BRIEF.md). Until a world is
  deployable and updatable by mundos, nothing below moves (87).

## The core: the sphere

### The shape

- [ ] Fix the generated shape: only the Earth one reads right today. Three named faults, all in `plates.rs`
  - [ ] `Plates::shape` takes no footprint at all, so a coarse patch reads the field at full detail and the shore lands somewhere else at every level: from orbit the coastline comes out in straight steps along the patch edges
  - [ ] The coast and belt noise are raw `fbm` where the rest of the generator is band limited `filtered`: the same root, and two lines
  - [ ] No shelf. Oceanic crust sits far enough below the blend that `deep` saturates everywhere, so the sea is a bathtub at the whole of `ocean_depth_m` with no shelf and no slope. The Earth side got `sea_curve` for this (45); this side needs its own answer
- [ ] Golden hashes also run on WASM in CI (wasmtime)
- [ ] Generator as WASM inside Go (wazero), when the server has to read the ground (09)
- [ ] Hydraulic erosion and rivers for the generated source: a coarse bake in `Generator::new`, dendritic valleys under the noise
- [ ] Climate as a field of its own: latitude bands and rain shadow, so deserts and rainforests land where they belong

### What an admin picks when a world is founded

- [ ] **Size.** A power of two per sector side, up to `2^16`, so a world can be a moon, an island or the planet we have. `sector_bits` is in the recipe (49, 50); the founding screen does not offer it yet. Growing past `2^16` is a separate question with its own numbers (45)
- [ ] **A list of generators, not a pair.** `Source` already names one; the founding screen offers the ones this version has, and a new one is a module beside `plates` and `field` without the body of v3 knowing
- [ ] **More fields than Earth.** The Moon and Mars from the same bake, a region of Earth at a kinder scale. Same code path, a different file
- [ ] **Climate and soil as choices.** `material` today is one function: latitude, height, and a noise for moisture. A world should be able to be arid, frozen, tropical or drowned and say so at its founding
- [ ] **Which plugins are on** (91)
- Each of these is in the address, previewed before it is kept, and frozen with the world, the plugins excepted

### The picture

- [ ] The far field as baked field textures rather than a generator call per patch: alt, ruggedness, horizon and colour per sector, frozen with the recipe. Measure the settle against the 7.4 s the pyramid took (57)
- [ ] A horizon map in the shader, so terrain self shadows at any range and nothing coarse enters a cascade (55, 57)
- [ ] Per pixel voxelization of the far ground, so it reads as the same world as the near cubes without being the same data (57)
- [ ] LOD by projected error with hysteresis; geomorph between levels. Nothing comes down before its replacement is up (53)
- [ ] Patch building behind a job queue (workers native and web), measured budget. Load bearing: one volume patch is 10.7 ms of a 12 ms frame
- [ ] Motion vector target
- [ ] Atmosphere with real scattering
- [ ] Snow and forest per pixel: continuous, footprint filtered cover fields in the vertex, the threshold in the shader
- [ ] Caustics; the waterline when the camera straddles the surface; reflections of terrain on the sea
- [ ] MSAA or filtered edges, after measuring cost
- [ ] The sphere leaves the seam: `scene::Frame` names the planet four times (radius, moon, sun, sea) and the own-indices mesh is the exception in `render`. One `Sky` struct and the volume mesh as the primary shape, so a renderer with no sphere ignores one field (65)
- [ ] What it must not cost, measured and not assumed: the far shimmer (29, 41, 43), and the frame on a Pi and in a tab

### Where you are

- [ ] Minimap: the ground around the avatar off the coarse quadtree, north up, coast and seam drawn; a click reads a place out
- [ ] The engine starts with no world until the page's arrives: `Engine::create` builds and streams seed 1 behind the veil for the length of the field download (73)
- [ ] A field arrives coarse first: the pyramid written coarsest level first and parsed as it streams, so the veil lifts on a rough Earth within a second and the ground sharpens underfoot

## Plugins: native

### Chat (69)

- [ ] Extracted as the first plugin, cutting the host (BRIEF.md)
- [ ] A mute, over the permission hook

### Building (58, 75 to 86)

- [ ] Extracted: a turn in the frame, solids for the footing, the picture through `scene` (BRIEF.md)
- [ ] A stroke as an op on the socket: the host asks the permission hook with who, what and where, the plugin applies its rule, keeps it and an event tells everyone in the world (76, 93)
- [ ] Every op a tool makes has a command at the seam that asks for it by its parameters (94)
- [ ] Kept by volume (95): copy on first write, chunk versions, the plugin's store in the world folder (89)
- [ ] Op log; undo; a volume rolled back to an earlier moment (95). It grows without a ceiling and will outweigh the chunks long before they matter, so how it is kept is part of building it
  - [ ] **An op is a gesture, not a cell.** One stroke is one permission-checked op carrying its shape and its parameters, never the thousands of cells it wrote
  - [ ] **The log is not the world.** Stored chunks are, by copy on first write, so the log is never replayed to rebuild anything: it exists for undo, audit and rollback, which is what makes it safe to compact
  - [ ] **Two tiers.** Inside the undo window an op is kept whole, with the chunk versions it bumped. Past the window it collapses to a digest: who, when, which chunks, how many cells
  - [ ] **Chunk version retention is the real knob**: undo depth, and how far back a volume rolls, are bounded by how many versions of a chunk are kept, not by how many ops are
  - [ ] **Compaction runs in the world actor**, off the hot path, on a schedule
  - [ ] Measure before choosing the window: bytes per op, and bytes per stored chunk version after zstd
- [ ] Delta sync on reconnect; client chunk cache (SQLite native, OPFS web)
- [ ] A volume on a plot at the edge of a sector, and a build across the seam: the cells folded over it (77)
- [ ] A volume on the moon: a platform asked for there is laid as on the planet (today it is refused)
- [ ] First cut leftovers: a key that lays a platform; the undo chord and the last tool said once in `client` rather than in each shell; the volumes uniform buffer starts small and grows; gesture and trace tests across a chunk seam
- [ ] The light of a volume: flood fill sun baked at mesh time, so an interior is dark and a doorway a gradient; no leak where a volume meets the ground; a glow channel
- [ ] A volume streamed by proximity, a silhouette from afar; one under the sea
- [ ] The camera boom cut by a volume's cells, as it is by rock under the ground
- [ ] Ramp, wedge and half slab beside the cube (75)
- [ ] Brushes inside a volume: dig, add, smooth, flatten
- [ ] Blocky is a cosmetic toggle inside a volume, one viewer's choice, never the world's
- [ ] A second way of building, a proof of concept beside this one (BRIEF.md)

### Land (14, 95)

- [ ] Permission to build by volume, given to an account, a person's or an agent's: the second answer to the permission hook, and building never knows it (93)
- [ ] A panel where an admin gives it, takes it and sees who holds what
- [ ] Landlords (14): a holder who subdivides and names builders inside; drawn in world with a gizmo, translucent borders while building
- [ ] The Atlas, the unfolded-cube 2D map

### Avatars (28)

- [ ] Extracted: the core draws a figure for a body, the plugin says which, and a hook says what is offered
- [ ] A person's own VRM, once files are uploaded
- [ ] A swim clip (the fly clip stands in)
- [ ] Avatar changer: an entity you walk through that opens a dialog to pick an avatar
- [ ] Avatar dropzone: drop a VRM on the map to place an avatar others can take

## Plugins: wishes

- [ ] Files: uploads through the world server into the bucket folder (89), what entities and media stand on
- [ ] Entities: GLB with budgets, primitive parts, gizmos
- [ ] Images: URL or upload, client-made thumbnail and low version, proximity LOD, texture budget
- [ ] Video on the web (browser decoder), decoder budget, posters
- [ ] Wallets: the avatars and galleries a wallet holds, OBJKTs from Tezos; land deeds as tokens
- [ ] Vehicles: hover first, raycast wheels later. An abstract vehicle first, then a motorcycle in the manner of Akira, blocky, on one fat wheel; `E` to mount, `E` to leave. It travels on the smooth collision surface like a person does, never on cubes
- [ ] Destruction: ops and local debris; protected, ephemeral, permanent modes (16)
- [ ] Scale bands: giant, human, bug; secrets streamed only to the right scale
- [ ] Portals between places, scales and worlds; magic as a capability
- [ ] Gravity fields as placeable entities (19)
- [ ] Scripts, server side
- [ ] Agents (94): headless clients with a door of their own in mundos, permitted by volume as a person is
  - [ ] Perception: readings of what is around a body, the ground, the cells, who is near, where it may build
  - [ ] Building on their own: every op a tool makes, asked for by its parameters
  - [ ] Capture: a picture from a pose, at low resolution, by the headless renderer, under a permission of its own
- [ ] Animals and NPCs
- [ ] Rockets, satellites on rails, buildable orbital grids
- [ ] The moon as a voxel body (Cartesian ball topology): digging and building on it

## Other screens

- [ ] The desktop opens the socket: `bun run desktop <url>` and `bun run shot --world <url>`, a world by the address a browser shows
- [ ] A door for a device, in mundos: a code or a QR on a screen pairs a headset or a desktop with an account
- [ ] A client reads what a world speaks before it enters (91)
- [ ] A plugin's panel outside Svelte (OPEN.md)
- [ ] Strings generated from `en.ts` into a Rust module, so the one source of truth survives a second front end
- [ ] The desktop remembers its settings
- [ ] Raspberry Pi 5 build (KMS/DRM)
- [ ] Native video through GStreamer (shared with the vybe work)
- [ ] Quest and Pico: OpenXR, two views

## Wishes (unordered)

- [ ] Compatibility kept across versions: an old world on a new version, an old client in a new world (91)
- [ ] Grass self shadowing: tufts in the contact cascade only, same bend as the visible pass; root occlusion first, it is free
- [ ] Clouds: temporal reprojection to spend fewer samples; high cirrus; weather as a recipe param; shade on water
- [ ] Flattened grass trails with timed recovery
- [ ] Softer shadows: over the tent of 83, a penumbra that widens with distance from the caster
- [ ] Shadow cascades fitted to the view, or a fourth between the first two: a cube 35 m from the eye is smaller than a texel of the cascade that holds it (80)
- [ ] More of the compositor as knobs: cloud layer height, shadow quality in levels with off the lowest (84), a reduced preset for the Pi
- [ ] World snapshots anchored in Bitcoin; genesis inscription of the recipe
- [ ] Cinema mode: cloud render with neural rendering, streamed over WebRTC
- [ ] Curvature knob: morph flat <-> sphere in the vertex shader
