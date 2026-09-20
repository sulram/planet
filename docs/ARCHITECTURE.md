# ARCHITECTURE

Current shape. The why lives in DECISIONS.md. Open points live in OPEN.md.
Numbers marked (p) are proposed and not yet confirmed.

## Two planes, one binary

| Plane | Owns | Tech |
|---|---|---|
| Cold | accounts, worlds, volumes, roles, entity records, asset records, quotas, admin panel | PocketBase (Go library), its own SQLite |
| Hot | chunks, op log, presence, edits, streaming | our Go code, one `world.db` (SQLite, WAL) per world, binary WebSocket |

- One Go executable: PocketBase with our routes and WebSocket registered inside.
- Bridge: volumes and roles cached in memory at start; PocketBase hooks
  invalidate the cache. A revoke in the panel applies on the next block.
- No transaction spans both files. Permission decides, op log records.
- The world core talks to PocketBase through one small interface of ours.
- Server layout: `cmd/planet` (entry), `internal/world` (core, imports no
  PocketBase), `internal/cold` (all PocketBase glue), `migrations` (Go, applied
  on serve). PocketBase settings come from the env and are reapplied on start.

## Topology: quad sphere, single build band

- Six sectors (cube faces), each a square grid, projected onto the sphere with
  the tangent warp (p): grid coordinate `s` moves to `tan(s * pi / 4)` on the
  face. A block edge is 0.5 m at a sector centre, 0.35 m at the corners.
- Seams are computed, not tabled: a column is an integer point on a cube, a
  step over an edge is one vector sum, swaps and flips fall out of the frames.
- Address: `sector (0..5), u, v, h` then chunk index and block index.
- Block edge 0.5 m (p). `2^16` blocks per sector side (p): u and v fit in 16
  bits. Radius about 20.9 km, surface about 5,500 km2.
- Build band: about +-128 m around the surface (p), bedrock at the bottom.
  Inside it a column tapers 0.6%, so the band is a regular grid in practice.
- The 8 sector corners are zoned as nature. No volume may include them.
- Small bodies (moons, micro worlds) use a second topology: a Cartesian ball
  of cubes, diggable to the core. Both sit behind one `Topology` trait.
- On foot the planet reads as flat: horizon at about 260 m from eye height.
  The curve shows from altitude.

## Voxels: two layers, one grid

| Layer | Cell holds | Mesher | Edited with | Used for |
|---|---|---|---|---|
| Terrain | density + material | surface nets | brushes (dig, add, smooth, flatten, paint) | nature, roads, craters |
| Build | block type (palette) | greedy cubes + ramps, wedges, half slabs | place and remove blocks | architecture |

- Both live in the same chunk and address. Chunk 16x16x16 (p).
- Chunk blob: palette + bit-packed indices (build), quantized density
  (terrain), zstd. The same blob serves disk, wire and client cache.
- Each stored chunk carries a version number and its reduced LOD levels.

## The world is a recipe

- World = seed + params + generator version. "Create world" writes one row.
- Wire shape, shared unmapped by Rust, Go, TypeScript and the `worlds`
  collection: `{seed, generator_version, params}`. The seed is a u64 written
  as 16 lowercase hex digits (JSON numbers stop at 2^53).
- The recipe of a stored world is frozen by a validate hook, superusers too.
- Generator v2 (new worlds): eroded massifs, a sea floor, and
  `sample_at(direction, footprint_m)` which fades detail finer than the mesh
  that asks. Collision and saves use full detail. v1 stays frozen beside it.
- Generator v1: continents, ridged mountains and detail as 3D simplex noise;
  materials water, sand, grass, forest, rock, snow. Params: `relief_m`,
  `ocean_depth_m`, `continent_scale`, `sea_share`.
- `world.db` stores only modified chunks and the op log.
- Read path: stored chunk if present, else generate. One function, everywhere.
- First edit to a chunk: generate it, apply the edit, store the whole chunk.
- Op log: who, when, address, before, after. Gives undo, audit, per-user
  rollback and the snapshot hash.
- The generator is written once in Rust: native in clients, WASM in the
  browser, the same WASM inside Go through wazero (pure Go, no CGO).
- 3D noise sampled on the sphere: no seams, no projection distortion.

## Streaming and LOD

- On arrival the client asks: which chunks near me are stored, at what version?
- The server sends only those. Untouched terrain costs zero bandwidth.
- Rings of interest around the player; ring depth follows the bandwidth budget.
- Quadtree per sector for planetary LOD, ground to orbit. Today it is the whole
  terrain: heightfield patches of 32x32 quads with skirts, down to one vertex
  per block, a few built per frame, nearest and coarsest first. Surface nets
  chunks will replace the deepest levels near the player.
- Client cache: SQLite on native, OPFS in the browser.

## Identity and permissions

- Account by email: magic link in the browser, one-time code typed on native
  and headset (PocketBase OTP). Wallets are optional links, later.
- Visitor: anonymous, enters any world, walks and looks, never builds.
  `worlds` is publicly readable; `/play` and `/w/[id]` need no login.
- Sign up and sign in are one flow: the first code request creates the
  account (server hook), the first valid code verifies it. No passwords.
- The email carries a link `{APP_URL}/login/verify?otpId=&code=` and the code.
- Operator: global flag `users.operator`. Gates `/backoffice`. Only an
  operator changes it. `PLANET_OPERATOR_EMAIL` seeds the first one.
- Any signed in user creates worlds and owns them. Per world roles arrive in M4.
- Agent: API token issued by a responsible user.
- Users are global. Roles are per world.

| Role | Scope | Can |
|---|---|---|
| Admin | world | build anywhere; every edit still logged |
| Landlord | volume | build, subdivide, name landlords and builders inside |
| Builder | volume | build |
| Visitor | world | look |

- Volume: integer address box `(sector, u0..u1, v0..v1, h0..h1)`, inside one
  sector. Child fully inside parent. Siblings never overlap.
- For any block at most one deepest volume applies. Power flows down only.
- Revoking removes rights, never the work. Outside any volume only admins build.
- Limits: tree depth about 4 (p), minimum volume size (p).
- Collections: `worlds`, `volumes`, `volume_roles`, `entities`, `assets`.
  Hooks validate containment and non-overlap. R*Tree for point queries.

## Entities, assets, media

- Entity: GLB, primitive part, light, gravity field, portal, media frame,
  script. Anchored to an address + local offset, oriented in the tangent frame.
- Asset file: global, named by content hash, immutable, cacheable forever.
- Placement: belongs to a world and a volume, counts against a budget
  (triangles, texture size, bytes).
- Asset source is one of: external public URL (needs CORS for web visitors),
  or our storage (Hetzner Object Storage, S3 API).
- Heavy media uploads go straight to the bucket with a presigned URL and are
  read from the bucket URL. PocketBase keeps the record and small files.
- Thumbnail and low version are made by the owner's client at placement time
  and uploaded. The server never fetches third-party URLs and runs no ffmpeg.
- Storage quota per user (bytes used, bytes allowed). Data packages raise it.
- Media LOD: far = thumbnail, mid = low version, near and in view = original.
- Hard cap on concurrent video decoders per platform, ranked by screen size,
  distance and facing. Losers show the poster. Spatial audio from the nearest.
- Video rule: MP4, H.264, faststart, host with range requests.
- `VideoSource` seam: browser decoder on web, GStreamer on desktop and Pi,
  MediaCodec on Quest and Pico.

## Avatars

- An avatar is a VRM, named by an **asset reference**: a path under the asset
  root (`avatars/Kyle.vrm`) or an absolute URL (a user's own upload, M5). The
  client never tells them apart; nothing addresses an avatar by index.
- `assets/manifest.json` is the config of the instance's default set:
  `default_avatar`, `avatars` on offer, `clips` per gait. Edited by hand.
- Which avatar a person wears: the user's default avatar (future
  `users.avatar`), else the visitor's earlier choice (cookie), else a random
  one from the offer, which becomes the choice. A failed load wears
  `default_avatar`; the box figure covers the time nothing is loaded.
- Clips are authored once on a Mixamo rig and retargeted at load to the VRM
  humanoid (crate `avatar`), so every avatar shares every clip. Gaits: idle,
  walk, run, jump, fall, fly. VRM 0.x, one skin, PNG textures for now.
- Asset seam: the client does no IO. It queues requests by reference, the
  platform shell fetches (disk on desktop, `fetch` in the browser) and answers.
- `V` wears the next avatar on offer. The engine reports `avatar_changed`; the
  web app keeps it as the visitor's choice (`POST /avatar`).

## Sea, sky and light

- The terrain mesh is the real ground, sea floor included. A patch that dips
  under sea level also carries a water surface: same grid, same indices.
- Water is drawn last, blended, from both sides: per channel absorption by
  depth, Fresnel to the sky, foam at the shore, Snell's window from below.
- A camera under sea level sees through water as a medium (red dies first).
- Swimming is part of walking: in water too deep to stand you float at chest
  depth, `Space` leaps, `C` or looking down while moving dives, idle drifts up.
- Sky: a shell atmosphere (3.6 km), a sun, stars fixed to the world.
- Bodies: the planet and the moon share one terrain quadtree (`Body`). Patches
  are built around their body's centre; the renderer adds where the body is
  this frame. The moon orbits on rails, 160 km out, 8 km radius, craters from
  generator v2, no sea. Craters are searched in a cell grid per sample; the
  few basins are listed once per `Generator`.
- Sunlight at a point is what neither sphere shadows: night and eclipses.
- Three separate things hold an avatar. Its **site**: the body it is stored
  relative to, changed at the moon's sphere of influence (4 radii), so it
  rides the orbit. **Gravity**: turns continuously from planet to moon with
  distance; a fifth as strong on the moon. Its own **frame** (`frame_up`):
  turns toward gravity by rotation, fast on foot, in flight only near a
  surface. Flight goes where you look.
- Ground detail is procedural noise anchored to the planet: patch origins are
  wrapped to 1024 m in f64 on the CPU. Rock shows by slope, per pixel.

## Gravity

- Gravity is a field, decoupled from geometry. Shapes: sphere, box, parallel,
  more later. Range + priority; the highest-priority field containing you wins.
- Constant strength, direction only. The avatar's up vector eases to the
  opposite of gravity. Input is projected on the plane normal to gravity.
- The planet is one sphere field. Fields are entities: streamed, permissioned.

## Clients and UI

| Client | Shell | UI |
|---|---|---|
| Browser | `shell-web`, WASM + WebGPU, WebGL2 fallback, worker + OffscreenCanvas | Svelte + Bun: full builder and player modes |
| Desktop | `shell-desktop`, winit | minimal native UI |
| Raspberry Pi | `shell-desktop` on KMS/DRM | minimal |
| Quest, Pico | `shell-xr`, Android + OpenXR + Vulkan | minimal |

- One command/event seam between core and any UI. Svelte panels send commands
  and render events. Tool logic stays in Rust so every client shares it.
  JSON tagged by `type`: `client::Command`, `client::Event`.
- Crates: `topology` and `worldgen` (deterministic, `libm`), `scene` (plain
  data a client hands a renderer), `avatar` (VRM + clips, no GPU), `client`, `render`, `shell-desktop`,
  `shell-web`. `voxel` and `protocol` appear when a milestone pulls them.
- The controller keeps its state in address space; a wish direction in metres
  becomes an address delta through the local tangents. Tangent vectors are
  parallel transported, so seams and corners need no special case.
- Web app: SvelteKit on adapter-node. One PocketBase client per request,
  session in the httpOnly `pb_auth` cookie, operator barrier in
  `hooks.server.ts`. It holds no superuser credentials.
- The WASM client lands in `apps/web/src/lib/engine/pkg` (`bun run wasm`),
  loaded by glob so the app builds without it.
- Design system: `apps/web/src/lib/ds`, catalogue at `/ds`. JetBrains Mono,
  one 10px size, black and white plus one red, radius 0, light and dark.
  Components reference semantic tokens only.
- i18n: flat dotted keys, `en.ts` is the source, `pt.ts` must match it.
- The renderer accepts N views from day one (1 desktop, 2 XR).
- The renderer writes depth (reversed, infinite) today; the motion vector
  target is on the ROADMAP (M1).

## Future shapes already accounted for

- Scale bands (giant, human, bug): the octree descends into sub-grids; the
  server streams only the levels your scale may see. Secrets are enforced by
  the server, not by the client.
- Orbits on rails: satellite position is a function of time. No sync needed.
- Destruction: the instigator predicts locally and sends an op; the server
  validates and broadcasts the event with a seed; debris is local and cosmetic.
  Per-volume mode: protected, ephemeral (regrows), permanent.
- Vehicles: hover and flight first; wheels by raycast with a smoothed
  collision surface later.
- SaaS: one world = one file; many worlds per instance; instance per tenant.
