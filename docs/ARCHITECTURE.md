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
