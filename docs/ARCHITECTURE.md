# ARCHITECTURE

Current shape. The why lives in DECISIONS.md. Open points live in OPEN.md.
Numbers marked (p) are proposed and not yet confirmed.

## Two planes, one binary

| Plane | Owns | Tech |
|---|---|---|
| Cold | accounts, worlds, volumes, roles, entity records, asset records, quotas, admin panel | PocketBase (Go library), its own SQLite |
| Hot | chunks, op log, presence, edits, streaming | our Go code, one `world.db` (SQLite, WAL) per world, binary WebSocket |

- One Go executable: PocketBase with our routes and WebSocket registered inside.
- The hot plane's doors, on PocketBase's router (`internal/cold/hot.go`):
  `POST /api/planet/ticket` mints a ticket for the signed in caller;
  `GET /api/planet/worlds/{id}/socket?ticket=` opens the world socket. No
  ticket is a visitor. A ticket works once and for a minute.
- Wire: protobuf, one message per binary WebSocket frame, `proto/` the single
  source (DECISIONS 66). `Hello` says the protocol version; any other is
  refused. `Welcome` carries the recipe, so a client checks it stands in the
  world the server holds.
- Presence: the world actor keeps every session's last stance and relays what
  changed at 15 Hz, in one frame encoded once, with a heartbeat every two
  seconds so a silent link is a dead one on both sides. A client too slow to
  take its frames is dropped; the actor never waits for a client.
- Chat: `Say` up, `Said` down, relayed by the actor to everyone in scope,
  the speaker included, and never stored (DECISIONS 69). `near` reaches
  `NearBlocks` on the same body, measured by the actor with the client's own
  projection mirrored in `near.go`; `world` reaches every body. A line said
  with `here` comes back with the speaker's stance as the actor holds it.
  Limits live in the actor: `LineChars`, five lines in five seconds.
- The actor holds no state past its sessions yet: `world.db` arrives with
  chunks (ROADMAP M3).
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
  `worlds` and `instance` are publicly readable; `/` and `/w/[id]` need no login.
- The instance opens on its main world: `/` is that world (DECISIONS 70),
  `/w/[id]` is any world by link, unlisted. The `instance` collection holds
  one record, seeded on start, a second refused by a hook; `main_world` is
  chosen on `/backoffice/worlds` and cleared by PocketBase if that world is
  deleted, so `/` then says the instance is not open yet.
- Signing in happens in the world: a dialog in the panel posts to the
  `/login` and `/login/code` actions and never leaves the page; the link in
  the email lands on `/login/verify` and returns to `/`.
- A name is set where it is shown, in the world panel: `users.name` for a
  signed in person, a cookie for a visitor, and `Rename` on the socket at
  once. The ticket's name wins over what Hello says, so an account cannot be
  impersonated by a client. At most `NameChars`.
- Sign up and sign in are one flow: the first code request creates the
  account (server hook), the first valid code verifies it. No passwords.
- The email is the server's, rendered in the reader's language: the link
  `{APP_URL}/login/verify?otpId=&code=` as a button and the code in large
  digits. The web app sends the locale with the code request and the account
  remembers it (`users.locale`, hidden); an account that never said one
  reads English.
- Operator: global flag `users.operator`. Gates `/backoffice`. Only an
  operator changes it. `PLANET_OPERATOR_EMAIL` seeds the first one. An
  operator reads every account's email; anyone else reads their own alone,
  which is PocketBase's default with `emailVisibility` off.
- Operators create worlds, on `/backoffice/explore` (the Worlds page's create button), and own them. Per world roles arrive in M4.
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
- Where you stand is the client's, never the page's (62). A new recipe keeps
  your place while the new ground is dry under it and spawns you otherwise.
  The address bar's place is honoured each time a hand puts one there, once
  the engine stands in the world the page asked for; what the page writes
  there as you move never comes back. Until that world stands and is drawn
  where you land (`settled`), a veil covers the picture (73).
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
  data a client hands a renderer), `avatar` (VRM + clips, no GPU), `protocol`
  (the wire, generated), `client`, `render`, `shell-desktop`, `shell-web`.
  `voxel` appears when volumes are built.
- The link is the shell's, the protocol the client's: `shell-web` opens the
  socket (`Engine.connect(url)`), hands every frame to `Client::receive` and
  sends what `drain_outbound` queues. The client says hello, keeps the peers,
  sends its own stance when it changed and as a heartbeat, and reports
  `session`, `peers`, `said` and `anchors` events over the seam, and
  `settled` when the streamer has nothing left to build for the view; `say` and
  `go_to` are the chat bar's commands. `anchors` is where every head in view
  is on the screen, each frame, so nametags and balloons are a front end's
  DOM and never a render feature. The desktop shell has no
  socket yet (ROADMAP M2).
- A peer is drawn a tick and a half behind its newest stance, between the
  last two heard, in world space: a walk across a seam never interpolates
  through the seam. Every body, the player's included, is one `Figure` over
  one shared set of clips; avatars load once per asset reference and are
  worn by any number of bodies (`client::wardrobe`).
- Web: `/w/[id]` hands the engine the socket URL and, for a signed in person,
  a path that mints a fresh ticket before every connection, so a reconnect is
  never a visitor by accident. The page reconnects with a doubling wait from
  one second to thirty. `PB_PUBLIC_URL` is where a browser reaches the server.
- The controller keeps its state in address space; a wish direction in metres
  becomes an address delta through the local tangents. Tangent vectors are
  parallel transported, so seams and corners need no special case.
- Web app: SvelteKit on adapter-node. One PocketBase client per request,
  session in the httpOnly `pb_auth` cookie, operator barrier in
  `hooks.server.ts`. It holds no superuser credentials.
- The WASM client lands in `apps/web/src/lib/engine/pkg` (`bun run wasm`),
  loaded by glob so the app builds without it.
- Design system: `apps/web/src/lib/ds`, planet's own, grown for the
  metaverse (DECISIONS 74); catalogue at `/ds`. JetBrains Mono, one 10px
  size, black and white plus one red, radius 0, light and dark. Components
  reference semantic tokens only.
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
