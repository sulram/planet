# ROADMAP

Intent, not contract. Add wishes freely, reorder as priorities shift, check a
box when it ships, strike what we drop and log the why in DECISIONS.md.
Every milestone ends runnable end to end.

## M0: the repo stands

- [ ] Cargo workspace, Go module, `apps/web` (Svelte + Bun), `proto/`, `scripts/`
- [ ] `CLAUDE.md`, `AGENTS.md -> CLAUDE.md`, `docs/`, `.gitignore` with `refs/`
- [ ] Conventional Commits check + Semantic Release
- [ ] CI builds native and WASM; runs tests
- [ ] A triangle on desktop and in the browser from the same `render` crate

## M1: walk and fly a generated planet, offline

- [ ] `topology`: address, neighbours, sector seams, address <-> position, property tests
- [ ] `worldgen`: layered 3D noise on the sphere, params as knobs, golden hashes native = WASM
- [ ] Terrain layer + surface nets mesher, in address space
- [ ] Camera-relative rendering, reversed-Z depth, quadtree LOD ground to orbit
- [ ] Controller: walk with radial gravity and auto-step, fly (superman), smooth up-vector
- [ ] Atmosphere shader, sun as rotating directional light
- [ ] Headless render to PNG (fixed clock + seed)
- [ ] Browser build with a minimal Svelte panel: seed, knobs, regenerate

## M2: create world, two people see each other

- [ ] `server/`: PocketBase embedded, `worlds` collection, "create world" writes the recipe
- [ ] `world.db` per world, world actor per active world
- [ ] Email login: magic link (web), one-time code (native); anonymous visitor
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

- [ ] Moon as a second body (Cartesian ball), travel between bodies
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
