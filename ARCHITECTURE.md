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

## Topology: quad sphere, single build band

- Six sectors (cube faces), each a square grid, projected onto the sphere with
  a pre-distorted mapping to keep blocks near square.
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
- Quadtree per sector for planetary LOD, ground to orbit.
- Client cache: SQLite on native, OPFS in the browser.

## Identity and permissions

- Account by email: magic link in the browser, one-time code typed on native
  and headset (PocketBase OTP). Wallets are optional links, later.
- Visitor: anonymous, walks and looks, never builds.
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
- The renderer accepts N views from day one (1 desktop, 2 XR).
- The renderer writes depth and motion vectors from day one.

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
