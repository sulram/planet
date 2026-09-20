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
- [ ] Terrain layer + surface nets mesher, in address space
- [x] Camera-relative rendering, reversed-Z depth, quadtree LOD ground to orbit
- [ ] Motion vector target; patch building on worker threads
- [x] Controller: walk with radial gravity and auto-step, fly (superman), smooth up-vector
- [x] Sun as rotating directional light; first atmosphere (uniform shell)
- [ ] Atmosphere with real scattering
- [x] Atmospheric clouds: a volumetric layer seen from the ground, from above and from orbit, with shadows on the ground
- [x] Headless render to PNG (fixed clock + seed)
- [x] Browser build with a minimal Svelte panel: seed, regenerate
- [ ] Params as knobs in the panel
- [x] Third person placeholder avatar (boxes, walk cycle)
- [x] VRM avatars with shared locomotion clips; random per visitor; `V` for the next
- [ ] User avatars: `users.avatar` default, own VRM uploads (with M5 assets)
- [x] Design system v0: JetBrains Mono 10px, black and white, light and dark, `/ds`

## M1.5: a planet worth looking at (from docs/TERRAIN_RENDER_BRIEF.md)

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
- [x] Ground detail anchored to the planet (procedural first, textures with provenance later)
- [x] Lighting: sky and ground ambient, tone mapping
- [x] Sun shadow cascades: terrain and avatars cast and receive, contact to horizon
- [x] Grass: instanced tufts in reach tiers to 640 m, wind in the vertex shader, bends away from the avatar
- [ ] LOD by projected error with hysteresis; geomorph between levels
- [ ] Patch building behind a job queue (workers native and web), measured budget
- [ ] MSAA or filtered edges, after measuring cost

## M2: create world, two people see each other

- [x] `server/`: PocketBase embedded, `worlds` collection, "create world" writes the recipe
- [ ] `world.db` per world, world actor per active world
- [x] Email login: magic link (web); anonymous visitor
- [ ] One-time code login on native
- [x] Backoffice for operators: worlds and users
- [ ] Binary WebSocket protocol; presence; avatars (default set)
- [ ] Generator as WASM inside Go (wazero)

## M3: build and dig, persisted

- [ ] Build layer: cubes + ramp, wedge, half slab; greedy mesher
- [ ] Terrain brushes: dig, add, smooth, flatten
- [ ] Copy on first write, chunk versions, reduced LOD levels on save
- [ ] Op log; undo; per-user rollback
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
- [ ] Vehicles: hover first, raycast wheels later
- [ ] Destruction: ops + local debris; protected, ephemeral, permanent modes
- [ ] Scale bands: giant, human, bug; secrets streamed only to the right scale
- [ ] Portals between places, scales and worlds; magic as a capability
- [ ] Gravity fields as placeable entities
- [ ] Scripts, server side
- [ ] AI agents as headless clients with API tokens
- [ ] Animals and NPCs
- [ ] Wallet linking; OBJKT galleries from Tezos; land deeds as tokens
- [ ] World snapshots anchored in Bitcoin; genesis inscription of the recipe
- [ ] Data packages (paid quota); managed hosting
- [ ] Cinema mode: cloud render with neural rendering, streamed over WebRTC
- [ ] Curvature knob: morph flat <-> sphere in the vertex shader
