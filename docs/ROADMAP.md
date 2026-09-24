# ROADMAP

Intent, not contract. Add wishes freely, reorder as priorities shift, check a
box when it ships, strike what we drop and log the why in DECISIONS.md.
Every milestone ends runnable end to end.

## M0: the repo stands

- [x] Cargo workspace, Go module, `apps/web` (Svelte + Bun), `scripts/` (`proto/` waits for the wire protocol, M2)
- [x] `CLAUDE.md`, `AGENTS.md -> CLAUDE.md`, `docs/`, `.gitignore` with `refs/`
- [x] Conventional Commits check + Semantic Release
- [x] CI builds native and WASM; runs tests
- [x] ~~A triangle~~ A planet on desktop and in the browser from the same `render` crate

## M1: walk and fly a generated planet, offline

- [x] `topology`: address, neighbours, sector seams, address <-> position, property tests
- [x] `worldgen`: layered 3D noise on the sphere, params as knobs, golden hashes
- [ ] Golden hashes also run on WASM in CI (wasmtime)
- [x] ~~Terrain layer + surface nets mesher, in address space~~ Twice superseded: first by a volume bolted to the deepest quadtree level, which cannot hold a world with an inside (48), then by voxels at every level, which was measured and is not payable on a Pi and in a tab. Nature is a surface and voxels live inside volumes (58, docs/BRIEF.md). What shipped and survives: collision reading the generator rather than a mesh (47), and the size of a world being a recipe value (49, 50)
- [ ] **Blocky is a cosmetic toggle** inside a volume, one viewer's choice, never the world's. Its greedy face merging is the build layer's mesher of M3, so it is written once
- [x] Camera-relative rendering, reversed-Z depth, quadtree LOD ground to orbit
- [ ] Motion vector target; patch building on worker threads
- [x] Controller: walk with radial gravity and auto-step, fly (superman), smooth up-vector
- [x] Sun as rotating directional light; first atmosphere (uniform shell)
- [ ] Atmosphere with real scattering
- [x] Atmospheric clouds: a volumetric layer seen from the ground, from above and from orbit, with shadows on the ground
- [x] Headless render to PNG (fixed clock + seed), and `--slice M` for a vertical cut through the density, which is the only way to look at a cave
- [x] Browser build with a minimal Svelte panel: seed, regenerate
- [x] Generator v3: the shape is a source; plates for a seed world, a baked field for a real one (DECISIONS 44)
- [x] "Earth or generated" when a planet is made, in `/play` and in the recipe
- [ ] Fix the generated shape: only the Earth one reads right today. Four named faults, all in `plates.rs`
  - [ ] **The wall.** `match (near.continental, far.continental)` switches branch on the bisector between two plates, and the two mixed cases are not each other: one adds `0.16 * force` to `land`, the other takes `0.85` away. So `land` steps by about 1 along every ocean to continent boundary, and the ground stands up in a cliff with the patch skirts showing through it as a picket fence. A v3 regression: v2 had no such branch. The asymmetry itself is right (the trench belongs to the ocean side and the arc to the continent side); it has to be a blend on the same `across` the crust level already uses, so both sides weigh a half on the line and nothing steps
  - [ ] No test caught it. `moon_has_no_cliffs` walks the moon and asserts no jump; the planet has no equivalent, and that is the test that would have failed the day the branch was written
  - [ ] `Plates::shape` takes no footprint at all, so a coarse patch reads the field at full detail and the shore lands somewhere else at every level: from orbit the coastline comes out in straight steps along the patch edges. A different fault from the wall, and the smaller of the two
  - [ ] The coast and belt noise are raw `fbm` where the rest of the generator is band limited `filtered`: the same root, and two lines
  - [ ] No shelf. Oceanic crust sits far enough below the blend that `deep` saturates everywhere, so the sea is a bathtub at the whole of `ocean_depth_m` with no shelf and no slope. The Earth side got `sea_curve` for this (DECISIONS 45); this side needs its own answer
- [x] Params as knobs in the panel, and in the address, so a planet stays shareable
- [x] The pose in the URL, replaced as the avatar settles and read on arrival, and `--at` taking one: body, place, height and aim, `m4-K7M42Q@40,180,-5`. A link opens where you stood looking at what you looked at, which is how a gallery gets shared. A respawn stands you back where you were, so turning a knob keeps your spot
- [ ] Minimap: the ground around the avatar off the coarse quadtree, north up, coast and seam drawn; a click reads a place out. The whole Atlas waits for M4
- [x] A coordinate system and a compass on screen, as Cryptovoxels has: `4-K7M42Q`, a prefix is a box and length is precision, plus a sixteen point bearing and the altitude. In `topology`, so every front end says the same thing, and the rose is i18n because N and S are English
- [x] Third person placeholder avatar (boxes, walk cycle)
- [x] VRM avatars with shared locomotion clips; random per visitor; `V` for the next
- [ ] User avatars: `users.avatar` default, own VRM uploads (with M5 assets)
- [x] Design system v0: JetBrains Mono 10px, black and white, light and dark, `/ds`

## M1.5: a planet worth looking at

- [x] Generator v2: eroded mountains, sea floor relief, LOD filtered sampling
- [x] Water as its own surface: Fresnel, depth colour, waves; the sea is penetrable
- [x] Underwater: fog, the surface seen from below, the sea floor
- [x] Swim at the surface, leap, dive by key or by looking down
- [ ] A swim clip (the fly clip stands in)
- [x] Night sky: stars fixed to the world, a moon on rails with real phases, moonlight
- [x] Water refraction and absorption from the scene depth and colour
- [ ] Caustics; the waterline when the camera straddles the surface; reflections of terrain on the sea
- [x] Water receives light like the land: cast shadows (terrain, avatars, clouds) on its surface and on the sea floor
- [x] From under water, the world outside: relief and sky refracted through Snell's window, fog that thins toward the surface instead of hiding everything. The scene target and its depth are there for it (compositor)
- [x] Material contract: explicit gloss, rock by slope in the shader, stable across LOD
- [x] The shore per pixel, by height: no mesh teeth on far coasts (DECISIONS 41)
- [ ] Snow and forest per pixel too: continuous, footprint filtered cover fields in the vertex, the threshold in the shader
- [x] Ground detail anchored to the planet (procedural first, textures with provenance later)
- [x] Lighting: sky and ground ambient, tone mapping
- [x] Sun shadow cascades: terrain and avatars cast and receive, contact to horizon
- [x] Grass: instanced tufts in reach tiers to 640 m, wind in the vertex shader, bends away from the avatar
- [ ] LOD by projected error with hysteresis; geomorph between levels
- [ ] Patch building behind a job queue (workers native and web), measured budget. Now load bearing: one volume patch is 10.7 ms of a 12 ms frame, so the volume's depth is bounded by the frame it is built in
- [ ] MSAA or filtered edges, after measuring cost

## M1.75: nature is a surface, building is volumes (from docs/BRIEF.md)

- The terrain layer is a heightfield again and not editable in world; cubic
  voxels live only inside a volume. Decided in DECISIONS 58; the brief carries
  the order of work and the numbers to beat.
- [x] World size in the recipe: `sector_bits` per world, `4..=16`, frozen like the generator version. A size is the scale the shape is printed at, so a small world is the same world (49, 50). `--bits N` on the desktop shell
- [x] The heightfield restored on top of it: the quadtree, the sea, the grass, the moon. `crates/voxel` and `crates/terrain` deleted, `client::collision` reading the ground rather than a density column, `client::tests::caves` gone with the feature
- [x] The peaks: a coast range came out as a plateau with a wall, because the gate on the relief was read at the mesh's resolution and amplified the source's own gradient. On land, p99 slope 93.8% to 65.8% and max 160% to 88%, with the range keeping its height (60)
- [ ] Fix the generated shape: the four faults in `plates.rs` named under M1
- [ ] The far field as baked field textures rather than a generator call per patch: alt, ruggedness, horizon and colour per sector, frozen with the recipe. Measure the settle against the 7.4 s the pyramid took (57)
- [ ] A horizon map in the shader, so terrain self shadows at any range and nothing coarse enters a cascade (55, 57)
- [ ] Per pixel voxelization of the far ground, so it reads as the same world as the near cubes without being the same data (57)
- [ ] LOD by projected error with hysteresis; geomorph between levels. Nothing comes down before its replacement is up (53)
- [ ] The volume: draw one, stamp the ground under it, greedy mesh it, bake its light, stream it by proximity, put one under the sea
- [ ] What it must not cost, measured and not assumed: the far shimmer (29, 41, 43), and the frame on a Pi and in a tab
- [ ] The sphere leaves the seam: `scene::Frame` names the planet four times (radius, moon, sun, sea) and the own-indices mesh is the exception in `render`. One `Sky` struct and the volume mesh as the primary shape, so a renderer with no sphere ignores one field (65)

## M2: create world, two people see each other

- [x] `server/`: PocketBase embedded, `worlds` collection, "create world" writes the recipe
- [x] World actor per active world (DECISIONS 66)
- [ ] `world.db` per world, opened by the actor
- [x] Email login: magic link (web); anonymous visitor
- [ ] One-time code login on native
- [ ] The cold plane moves behind the command and event seam: `list_worlds`, `create_world`, `enter_world`, with the clamping `+page.server.ts` does today living in `client` where both front ends reach it (DECISIONS 46)
- [ ] `bun run desktop <url>` and `bun run shot --world <url>`: a world opened by the same address the browser shows, a bare id meaning the configured instance. Needs the seam, an HTTP client and a local cache of recipes
- [ ] An instance's main world, in the cold plane beside `worlds`, so the web and the desktop open the same front door
- [ ] Desktop screens over that seam: sign in, a list of worlds, enter one, preview a planet with its knobs and create from it. The backoffice stays on the web
- [ ] Strings generated from `en.ts` into a Rust module, so the one source of truth survives a second front end
- [x] Backoffice for operators: worlds and users
- [x] Binary WebSocket protocol; presence; avatars (default set) (DECISIONS 66)
- [ ] Presence on the desktop: `bun run desktop <url>` opens the same socket, once the seam item above gives it the recipe
- [ ] Names in the world: a nametag over each peer, and a name a person sets on their account
- [x] Chat: `Say` and `Said` on the socket, scopes `near` and `world`, `@here` shares a place to teleport to; limits in the actor (DECISIONS 69)
- [ ] Generator as WASM inside Go (wazero)

### The world generator: what a person picks when a world is made

- [ ] **Size.** A power of two per sector side, up to `2^16`, so a world can be a moon, an island or the planet we have. Capping at today's maximum is what makes it cheap: no integer widens, `u` and `v` still fit. The cost is `topology` carrying the value instead of knowing it at compile time, and the generator quoting its wavelengths against the radius it is handed. Growing past `2^16` is a separate question with its own numbers (OPEN.md, DECISIONS 45)
- [ ] **A list of generators, not a pair.** `Source` already names one; the create page offers the ones this instance has, and a new one is a module beside `plates` and `field` without the body of v3 knowing. `earth` reads right today, `generated` is the one being fixed above
- [ ] **More fields than Earth.** The Moon and Mars from the same bake, a region of Earth at a kinder scale. Same code path, a different file
- [ ] **Climate and soil as choices.** `material` today is one function: latitude, height, and a noise for moisture. A world should be able to be arid, frozen, tropical or drowned and say so at creation, which means a climate a recipe can name and a soil that follows it. Water share is already a knob and is the first of them
- [ ] Every one of these is a recipe param, so it is in the address, previewed before it is saved, and frozen with the world

## M3: build and dig, persisted

- [ ] Build layer: cubes + ramp, wedge, half slab; greedy mesher
- [ ] Terrain brushes: dig, add, smooth, flatten
- [ ] Copy on first write, chunk versions, reduced LOD levels on save
- [ ] Op log; undo; per-user rollback. It grows without a ceiling and will outweigh the chunks long before they matter, so how it is kept is part of building it
  - [ ] **An op is a gesture, not a cell.** One brush stroke is one permission-checked op carrying its shape and its parameters, never the thousands of cells it wrote. This is the only lever that bounds the volume at the source, and it is what makes an op cheap enough to keep
  - [ ] **The log is not the world.** Stored chunks are, by copy on first write, so the log is never replayed to rebuild anything: it exists for undo, audit and rollback. That is what makes it safe to compact, and it should be written down where someone will read it before they build on it
  - [ ] **Two tiers.** Inside the undo window an op is kept whole, with the chunk versions it bumped, and undo is restoring those versions. Past the window it collapses to a digest: who, when, which chunks, how many cells. Moderation and rollback still work; per cell before and after does not survive
  - [ ] **Chunk version retention is the real knob**, because undo depth is bounded by how many versions of a chunk are kept, not by how many ops are. Keep the last few, or those newer than the window
  - [ ] **Compaction runs in the world actor**, off the hot path, on a schedule. The hot plane never waits for it
  - [ ] Measure before choosing the window: bytes per op, and bytes per stored chunk version after zstd. Those two numbers decide the rest
- [ ] Delta sync on reconnect; client chunk cache (SQLite native, OPFS web)

## M4: land

- [ ] `volumes`, `volume_roles`; containment and non-overlap hooks
- [ ] In-memory permission cache with hook invalidation
- [ ] Roles: admin, landlord, builder, visitor
- [ ] Draw a volume in-world with a gizmo; translucent borders in build mode
- [ ] Atlas: the unfolded-cube 2D map

## M5: things in the world

- [ ] Entities: GLB upload with per-volume budgets, primitive parts, gizmos
- [ ] Images: URL or upload, client-made thumbnail + low version, proximity LOD, texture budget
- [ ] Storage quota per user; Hetzner bucket with presigned uploads
- [ ] Video on the web (browser decoder), decoder budget, posters

## M6: more screens

- [ ] Raspberry Pi 5 build (KMS/DRM)
- [ ] Native video through GStreamer (shared with the vybe work)
- [ ] Quest and Pico: OpenXR, two views, code login

## Wishes (unordered)

- [ ] Grass self shadowing: tufts in the contact cascade only, same bend as the visible pass; root occlusion first, it is free
- [ ] Clouds: temporal reprojection to spend fewer samples; high cirrus; weather as a recipe param; shade on water
- [ ] Flattened grass trails with timed recovery
- [ ] Softer shadows: a wider rotated PCF kernel, penumbra that widens with distance from the caster
- [x] Compositor: HDR scene target, a chain of stages, one final tone map
- [x] Bloom as a compositor stage, haze and a choice of tone map as knobs
- [x] Settings panel over the command/event seam: Svelte on the web, egui on desktop, a button in the top right corner
- [ ] More of the compositor as knobs: cloud layer height, shadow quality, a reduced preset for the Pi; the desktop remembers the choice
- [ ] Avatar changer: an entity you walk through that opens a dialog to pick an avatar
- [ ] Avatar dropzone: drop a VRM on the map to place an avatar others can take

- [x] A moon you can fly to: a second body with cratered terrain, its own gravity, walk and jump on it
- [ ] The moon as a voxel body (Cartesian ball topology): digging and building on it
- [ ] Rockets, satellites on rails, buildable orbital grids
- [ ] Vehicles: hover first, raycast wheels later. An abstract vehicle first, then one that is a motorcycle in the manner of Akira, blocky, on one fat wheel; `E` to mount, `E` to leave. It travels on the smooth collision surface like a person does, never on cubes, which is why the blocky view stays cosmetic. This is on the "Do NOT add" list of M1 (CLAUDE.md), so pulling it forward is a decision to log in DECISIONS in the commit that does it, not something to let drift
- [ ] Destruction: ops + local debris; protected, ephemeral, permanent modes
- [ ] Scale bands: giant, human, bug; secrets streamed only to the right scale
- [ ] Portals between places, scales and worlds; magic as a capability
- [ ] Gravity fields as placeable entities
- [ ] Hydraulic erosion and rivers for the generated source: a coarse bake in `Generator::new`, dendritic valleys under the noise
- [ ] Climate as a field of its own: latitude bands and rain shadow, so deserts and rainforests land where they belong. What the climate choice in M2 stands on
- [ ] Scripts, server side
- [ ] AI agents as headless clients with API tokens
- [ ] Animals and NPCs
- [ ] Wallet linking; OBJKT galleries from Tezos; land deeds as tokens
- [ ] World snapshots anchored in Bitcoin; genesis inscription of the recipe
- [ ] Data packages (paid quota); managed hosting
- [ ] Cinema mode: cloud render with neural rendering, streamed over WebRTC
- [ ] Curvature knob: morph flat <-> sphere in the vertex shader
