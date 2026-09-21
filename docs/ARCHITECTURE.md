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
- `crates/voxel` carries the terrain blob today: a chunk is either one
  repeated cell, which most of a world is and which costs 4 bytes, or its
  cells. Density is the signed distance to the ground in cells, a byte over
  +-2 cells; the material palette spends one bit a cell where a chunk is rock
  and air, and none where the cover does not change. zstd wraps it as a second
  kind when the wire path arrives, which the format's version byte allows.
- Each stored chunk carries a version number and its reduced LOD levels.

## The world is a recipe

- World = seed + params + generator version. "Create world" writes one row.
- Wire shape, shared unmapped by Rust, Go, TypeScript and the `worlds`
  collection: `{seed, generator_version, params}`. The seed is a u64 written
  as 16 lowercase hex digits (JSON numbers stop at 2^53).
- The recipe of a stored world is frozen by a validate hook, superusers too.
- Generator v3 (new worlds): the shape is a source, and the body below it is
  v2's. `params.source` is `generated` (tectonic plates over the seed) or
  `{field}` (a baked cube map of a real body, named by content id).
- Generator v2: eroded massifs, a sea floor, and
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
- A field gives shape, never height: at 1/305 of Earth, honest elevations are
  a billiard ball and honest exaggeration is a wall. Zero maps to zero, so the
  coastline is exact; the relief is the generator's, sized by `relief_m`.
- Fields are baked by `bun run field` into `assets/fields/` (gitignored), each
  with a sidecar naming its content id. Default: ETOPO 2022, public domain,
  1024 texels per face side (32 m of planet, 9.8 km of Earth), 25 MB.
- Params are one table in `$lib/world.ts`: range, step, default and the shapes
  each means anything for. They are sliders in `/play` and parameters of its
  address; the server clamps again when a world is created. A knob at its
  default is absent from both.
- The sea takes an exaggeration of its own (`sea_curve`), because a depth in
  proportion to a real body leaves every strait a shoal, and `sea_level_m`
  moves a field's coastline the way `sea_share` moves a generated one.
- A recipe that names a field cannot be generated without it: the shell reads
  it (`--field` on desktop, one fetch on the web) and hands it over before the
  recipe. `Generator::new` refuses; `Generator::with_field` checks the id.

## Streaming and LOD

- On arrival the client asks: which chunks near me are stored, at what version?
- The server sends only those. Untouched terrain costs zero bandwidth.
- Rings of interest around the player; ring depth follows the bandwidth budget.
- Quadtree per sector for planetary LOD, ground to orbit. Today it is the whole
  terrain: heightfield patches of 32x32 quads with skirts, down to one vertex
  per block, a few built per frame, nearest and coarsest first. Surface nets
  chunks will replace the deepest levels near the player.
- The deepest quadtree level meshes its ground from the density by surface
  nets instead of from the height, so a cave, an arch and an overhang exist
  there. A patch of that level is exactly `2 x 2` chunks across and one block
  a cell. The level above is still a heightfield with its skirt, and the two
  agree on where the ground is, so the handover is the LOD boundary that was
  already there: nothing new streams and nothing is suppressed.
- A heightfield patch is `PATCH_VERTICES` in a grid every patch shares; a
  volume patch brings its own indices. The streamer's budget counts work
  rather than patches, because a volume patch costs about six of a height.
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
- The sea's waves are six octaves of drifting noise, a hand wide to a quarter
  of a kilometre long, anchored to the planet like the ground's detail. Each
  is kept only while a pixel can show it, as the generator keeps an octave
  only while the mesh can: near, all of them; from the sky, the long swell
  alone. What a pixel can no longer show becomes roughness, which widens and
  dims the sun's mirror, so a spark at hand is a road of light from above.
  One noise lookup an octave, value and slope together.
- The sea is drawn after the opaque world and the sky, in a pass of its own:
  the picture so far is copied aside and the water reads that copy and the
  depth (`Composer::behind`). It tests depth itself, having none attached.
  The side of the sphere turned from the camera (the far sea, past the
  horizon) is discarded: nothing else would hide it behind the near one.
- From above it refracts what lies under it and colours it by the water
  actually crossed (down to the floor, back along the ray), so shallows go
  turquoise. It takes sun, cast shadows and cloud shade as the land does.
- A camera under sea level sees through water as a medium (red dies first).
  What lies past the surface is drawn with its air alone; the surface lays the
  water between, and shows the world above straight through, bent by the
  waves' slope as an angle (so a far ridge swims as much as a near one); it
  mirrors the sea only at a glancing look. Only what lies past
  the surface along its own ray may be seen through it.
- Clouds and sea are drawn nearer last: clouds first for a camera under the
  sea, after it for any other. `water_clarity` stretches a swimmer's sight.
- Swimming is part of walking: in water too deep to stand you float at chest
  depth, `Space` leaps, `C` or looking down while moving dives, idle drifts up.
- What holds a body up is a **footing** (`client::collision`): the top of the
  solid at or under its feet, and the bottom of the solid over its head, read
  from one column of the density field. The surface out in the open, the
  cave's own floor and roof inside one. No mesh: a footing costs 2 us, and a
  step, which asks for four, costs 8 (DECISIONS 47).
- A rise of one block is taken in stride and two is a wall, to be jumped or
  flown, going up and coming down. A body needs its own height of room to walk
  into a place, unless it already has less, so a tight place is not a trap.
  Blocked, a step is tried along one address axis and then the other, which is
  what slides a body along a wall.
- Rock at knee height means the body is in rock rather than on it, and then
  the floor is the ground itself: a wall is not walked into, a buried body is
  let out upward, and nothing falls through the planet.
- Collision is the ground in full detail, never the filtered one: what a body
  stands on may not change with where the camera is. It reaches as deep as the
  field does, which is deeper than the volume is drawn.
- Inside the ground the third person boom is cut by the rock behind it instead
  of lifted over the terrain, so the camera stays in the cave with the body.
  Flight keeps its own floor over the drawn ground, except under the ground,
  where the footing takes over.
- Sky: a shell atmosphere (3.6 km), a sun, stars fixed to the world.
- Bodies: the planet and the moon share one terrain quadtree (`Body`). Patches
  are built around their body's centre; the renderer adds where the body is
  this frame. The moon orbits on rails, 160 km out, 8 km radius, craters from
  generator v2, no sea. Craters are searched in a cell grid per sample; the
  few basins are listed once per `Generator`.
- Sunlight at a point is what neither sphere shadows: night and eclipses.
- The compositor (`render::compose`): the world is drawn once into an HDR
  scene target (`Rgba16Float`, linear light) with its depth kept. A chain of
  full screen stages follows, each reading the colour and depth before it;
  the last, `output`, applies exposure, the chosen tone map (ACES, AgX,
  Khronos neutral, Reinhard, linear) and the target's encoding. No scene
  shader tone maps. An effect is a stage.
- Bloom: what is over a threshold (soft knee) is halved down a five level
  pyramid and summed back up it with a tent filter, then laid over the scene.
  After the clouds, so their silver edges glow too. The threshold is of
  exposed light. Haze is the air's density
  as a factor, in `atmosphere`.
- What glows is what is bright: bloom selects nothing. The moon in the night
  sky is drawn `MOON_SHINE` times a sunlit rock (`moon_shine` in
  `common.wgsl`), over the threshold: not the moon one stands on, nor the
  moon by day (`night_sky`, the rule that drowns the stars). Nothing about
  the picture adapts by itself.
- Clouds are a shell of weather, 1100 to 3000 m over the sea, made of one
  tiling 64^3 noise texture drawn once on the GPU (`render::clouds`). The
  density field (`cloud_field.wgsl`) is in every shader: the compositor
  marches it, and every lit surface asks it for shade along the sun.
- The march runs at half size from the scene depth (two paces: strides in
  clear air, short steps in cloud, both growing with distance), and a full
  size stage lays it over the scene, cut where terrain stands in front.
- At night clouds take the starlight and moonlight the land takes
  (`STARLIGHT`, `moonlight` in `common.wgsl`): pale over dark ground, never
  a hole in it.
- Weather turns about the planet's axis with the clock (the wind), and the
  noise rises through the layer along the local up, so clouds reshape in
  place as they travel. The angle wraps and the rise swings, both on the
  CPU in f64. Cosmetic: not simulated, not stored, the same for a clock.
- Sun shadows: three cascades around the eye (40 m, 400 m, 4 km half side,
  1024 px each), snapped to their texel, in the frame of the nearest body.
  Terrain, boxes and avatars cast; everything lit by `lit` receives.
- Casters are not the drawn patches: `Frame::shadow_patches` holds built
  leaves before view culling, coarser with distance (1 m, 4 m, 16 m), and
  never schedules generation. Skirts do not cast.
- Cascade count and size live in `render::shadow`, which prepends them to
  every shader; texel sizes ride the view uniform.
- Grass is cosmetic, planet only, on the meadow material alone (forest ground
  is a shade off it and bare), built with the patch from its own samples:
  no generator call. Tier `k` has one tuft per `2^k` half blocks and reaches
  `20 m * 2^k` (six tiers, 640 m), so screen density stays level.
- A tuft is (sector, tier, tier cell): subdivision never moves it. A patch
  carries only the tiers that can reach it before it splits, farthest first;
  the renderer draws the prefix in reach, near patches first, capped.
- `Frame::interaction` is one capsule (the avatar) that bends tufts. Visual
  only. `scene::Effects` turns shadows, grass and clouds off and tunes the clouds
  and exposure (Clients and UI).
- Three separate things hold an avatar. Its **site**: the body it is stored
  relative to, changed at the moon's sphere of influence (4 radii), so it
  rides the orbit. **Gravity**: turns continuously from planet to moon with
  distance; a fifth as strong on the moon. Its own **frame** (`frame_up`):
  turns toward gravity by rotation, fast on foot, in flight only near a
  surface. Flight goes where you look.
- Ground detail is procedural noise anchored to the planet: patch origins are
  wrapped to 1024 m in f64 on the CPU. Rock shows by slope, per pixel.
- The shore (sand, sea floor) shows by height over the sea, per pixel: a
  smooth contour at every LOD. A vertex under it carries the cover of the
  land beside it.

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
| Desktop | `shell-desktop`, winit | `ui-native`: egui settings panel |
| Raspberry Pi | `shell-desktop` on KMS/DRM | minimal |
| Quest, Pico | `shell-xr`, Android + OpenXR + Vulkan | minimal |

- One command/event seam between core and any UI. Svelte panels send commands
  and render events. Tool logic stays in Rust so every client shares it.
- Settings are `scene::Effects`: set with `set_effects`, clamped by the client,
  answered with `effects_changed`, carried in every `Frame`. The renderer holds
  no setting of its own. Both UIs put a button in the top right corner.
- Web: `engine/Settings.svelte`; the choice stays in the browser
  (`localStorage`), since it belongs to the machine, not the account.
- Desktop: `ui-native`, where all of egui lives. egui and egui-winit from
  crates, the painter ours (`egui-wgpu` pins an older wgpu). The shell hands it
  window events first while the pointer is free. `shot --panel` paints it over
  the headless picture. It does not persist the choice yet.
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
