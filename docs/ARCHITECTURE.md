# ARCHITECTURE

The shape as decided. The why lives in DECISIONS.md, open points in OPEN.md,
and how much of it is built in BRIEF.md. A line marked (p) is proposed and
not yet confirmed.

## A world, hosted

- An instance is one world (87): one container from a published version, one
  world folder that outlives it, one bucket folder, one port. mundos
  (`~/Dev/mundos`) creates, stops, addresses, lists and upgrades it; planet
  holds none of that. How it is hosted: DEPLOY.md.
- A world is born unfounded: no recipe. The first admin to enter previews a
  planet offline, turns its knobs and founds the world from what is on the
  screen; the recipe is frozen from then on. Until then anyone else is told
  the world is not made yet.
- A world answers for itself at `GET /api/world`: what it says about itself,
  for mundos's catalog and door, and what it speaks (91): the engine's
  version, the wire's, and the plugins that are on.
- An update is never in place: mundos copies the folder and the bucket folder
  into the next generation on the new version, and a promotion moves the
  address.

## Identity and levels

- Who is who is mundos's. The world stores no account, e-mail or password.
- Every load passes through mundos's door: a page with no `#identity=` in its
  address goes to `<mundos>/enter?host=<host>` and comes back with a token, or
  with `guest`. The pose in the address bar is kept across the hop.
- The token is a JWT signed with Ed25519 (`EdDSA`): `iss` `mundos`, `aud` the
  world's name, `sub` the account's id, `name`, `level`, `exp` a minute on,
  read with 30 s of leeway. The server checks it with `MUNDOS_PUBLIC_KEY`.
- The page hands the token to the world once, on entering, and gets a key:
  kept in the page's memory, shown on the socket's URL and on a founding, held
  by the server in memory for half a day. The level holds for the life of the
  page; a role changed in mundos holds on the next load.
- A link that drops opens again with the same key. A key the server no longer
  holds, after a restart or at the end of its life, sends the page through
  the door.

| Level | Who | May |
|---|---|---|
| `admin` | a superadmin of mundos, or an admin of this world | found the world, choose its plugins, build |
| `builder` | a builder of this world | build |
| `signed_in` | an account with no role here | walk, under the account's name |
| `anonymous` | no account | walk, under a name of their own choosing |

- `Welcome` says the session's level, and the client offers its tools by it.
  A stroke that is an op is checked by the actor against the same level,
  through the permission hook.
- With no `MUNDOS_PUBLIC_KEY` there is no door: every session is
  `PLANET_DEV_LEVEL`, `anonymous` unless set. `bun run dev` sets `admin`.

## The core and its plugins

- The core (88) is what a plugin stands on: the body (`topology`, `worldgen`,
  the recipe), the picture (`render`, `scene`), a body walking it (controller,
  footing on the ground, camera), the link (socket, session, level, a stance
  for each session) and the host of plugins. It runs with every plugin off.
- A plugin is a slice through up to four places: a crate in the client, a
  package in the server, a payload on the wire, a panel in each front end.
  Ours, compiled in: a config at the root lists them and the build bundles
  them, so a version is the core plus the plugins chosen for it.
- Native plugins: chat, avatars, building. Each is extracted from where it
  stands today, in that order (BRIEF.md).
- What the host offers, cut as each extraction asks for it: commands and
  events on the seam; a turn in each frame; solids for the footing; plain data
  for the picture through `scene`; an envelope on the wire, the plugin's name
  beside its payload; a store in the world folder; files; a place for a panel.
- A hook is where the core keeps a rule a plugin may change, written when a
  plugin asks: who may do what and where (the default is the level), which
  avatars are offered.
- A plugin imports the core, never the reverse, and never touches wgpu.
- Which plugins are on is the world's own (91). The config says whether each
  starts on; the admin switches any of them at the founding and after, in the
  world folder. A plugin switched off keeps its store untouched.

## The server

- One Go executable is the instance (90): the world socket, the world's
  routes, the files when there is no bucket, and the web front end as static
  files. `cmd/planet` is the entry, `internal/world` the core, `internal/api`
  the routes, and a plugin's server half is a package of its own.
- Wire: protobuf, one message per binary WebSocket frame, `proto/` the single
  source (66). `Hello` says the protocol version; any other is refused.
  `Welcome` carries the session, its level and the recipe, so a client checks
  it stands in the world the server holds.
- The hub holds the world's actor: started on the first session, gone after
  the last. The actor keeps every session's last stance and relays what
  changed at 15 Hz, in one frame encoded once, with a heartbeat every two
  seconds so a silent link is a dead one on both sides. A client too slow to
  take its frames is dropped; the actor never waits for a client.
- Routes: `GET /api/world` says what the world is, `POST /api/enter` trades
  the door's token for a key, `GET /api/me` says whether a key stands,
  `POST /api/world` is the founding, `GET /api/socket` the world socket. A
  route that changes the world takes the key and checks its level. An error
  is a code, never a sentence.
- Chat (69): `Say` up, `Said` down, relayed by the actor to everyone in scope,
  the speaker included, never stored. `near` reaches `NearBlocks` on the same
  body, measured with the client's own projection mirrored in `near.go`;
  `world` reaches every body. A line said with `here` comes back with the
  speaker's stance as the actor holds it. Limits live in the actor:
  `LineChars`, five lines in five seconds.
- The world folder (89): `world.json`, the recipe the founding froze, and one
  SQLite file for each plugin that keeps things, moved forward by that plugin
  when it starts. Permission decides, the plugin's op log records.

## Files

- A heavy file, an image, a video, a GLB, is named by the hash of its content
  and never changes (89). A record names it `asset://<hash>.<ext>`.
- A session that may build sends it to the world server, which names it and
  writes it to the bucket folder mundos named for this generation. Browsers
  read it from the address in front of the bucket. With no bucket the world
  folder holds the files and the server serves them. On start a world deletes
  from its bucket folder what no record names.
- The default asset set (avatars, clips, fields) is part of a version, served
  from the image; `assets/manifest.json` says what is in it.
- Entities and media are plugins to come: a GLB, a part, a light, a media
  frame, anchored to an address with a local offset, under a budget of
  triangles, texture size and bytes. The owner's client makes the thumbnail
  and the low version at placement; the server fetches no third-party URL and
  runs no ffmpeg. Far draws the thumbnail, mid the low version, near and in
  view the original, under a hard cap on videos decoding at once.
- Video rule: MP4, H.264, faststart, a host with range requests. `VideoSource`
  is one seam with a backend for each platform.

## Clients and UI

| Client | Shell | UI |
|---|---|---|
| Browser | `shell-web`, WASM + WebGPU | Svelte: the panels of the core and of each plugin |
| Desktop | `shell-desktop`, winit | `ui-native`: egui settings panel |
| Raspberry Pi | `shell-desktop` on KMS/DRM | minimal |
| Quest, Pico | `shell-xr`, Android + OpenXR + Vulkan | minimal |

- One command/event seam between the core and any UI: a front end sends
  commands and renders events, and tool logic stays in Rust so every client
  shares it. JSON tagged by `type`: `client::Command`, `client::Event`.
- A front end is only a front end (90). The web one is static files with
  no server half, so what any front end may do is the seam and the world's
  routes. Language, theme, avatar and a visitor's name are the browser's.
- Web: the page asks the world what it is, hands the engine the recipe and
  the socket's URL with its key, and shows the founding screen to an admin of
  an unfounded world. It reconnects with a doubling wait, a second to thirty.
- Where you stand is the client's, never the page's (62). A new recipe keeps
  your place while the new ground is dry under it and spawns you otherwise.
  The address bar's place is honoured each time a hand puts one there, once
  the engine stands in the world the page asked for; what the page writes
  there as you move never comes back. Until that world stands and is drawn
  where you land (`settled`), a veil covers the picture (73).
- Settings are `scene::Effects`: set with `set_effects`, clamped by the client,
  answered with `effects_changed`, carried in every `Frame`. The renderer holds
  no setting of its own. Both UIs put a button in the top right corner; the
  web keeps the choice in the browser, since it belongs to the machine.
- Building (76, 78, 79, 85): `set_tool` takes a tool or `null`, then
  `set_paint`, `set_platform`, `lay_platform` with its base, `undo`, `redo`;
  the engine says `tool_changed`, `palette`, `build_refused`, `history`. Both
  UIs put Build in the bottom right corner. Building, the pointer is free and
  `Input` carries where it is and whether Alt is held with it: the primary
  button is the tool's, the secondary one looks. 1 2 3 take a tool, Alt turns
  a stroke, Escape drops the stroke and then the tool, Cmd or Ctrl Z takes a
  stroke back and with Shift puts it back.
- Desktop: `ui-native`, where all of egui lives. egui and egui-winit from
  crates, the painter ours (`egui-wgpu` pins an older wgpu). The shell hands it
  window events first while the pointer is free; `shot --panel` paints it over
  the headless picture.
- The link is the shell's, the protocol the client's: `shell-web` opens the
  socket (`Engine.connect(url)`), hands every frame to `Client::receive` and
  sends what `drain_outbound` queues. The client says hello, keeps the peers,
  sends its own stance when it changed and as a heartbeat, and reports
  `session`, `peers`, `said` and `anchors` over the seam, and `settled` when
  the streamer has nothing left to build for the view. `anchors` is where
  every head in view is on the screen, each frame, so nametags and balloons
  are a front end's DOM and never a render feature. The desktop shell has no
  socket yet (ROADMAP § Other screens).
- A peer is drawn a tick and a half behind its newest stance, between the
  last two heard, in world space: a walk across a seam never interpolates
  through the seam. Every body, the player's included, is one `Figure` over
  one shared set of clips; avatars load once per asset reference and are
  worn by any number of bodies (`client::wardrobe`).
- The controller keeps its state in address space; a wish direction in metres
  becomes an address delta through the local tangents. Tangent vectors are
  parallel transported, so seams and corners need no special case.
- On foot the body takes a step at once and the camera's eye comes after on
  a spring, along up only; all else the camera follows with no play (81).
- Design system: `apps/web/src/lib/ds`, planet's own (74); catalogue at `/ds`.
  Components reference semantic tokens only. i18n: flat dotted keys, `en.ts`
  is the source, `pt.ts` must match it.
- The renderer accepts N views (1 desktop, 2 XR) and writes reversed depth.

## Future shapes already accounted for

- Another screen is a shell around the core, a door in mundos for a device
  (a code on a screen), and each plugin's panel in that front end (OPEN.md).
- Scale bands (giant, human, bug): the octree descends into sub-grids; the
  server streams only the levels your scale may see. Secrets are enforced by
  the server, not by the client.
- Destruction: the instigator predicts locally and sends an op; the server
  validates and broadcasts the event with a seed; debris is local and cosmetic.
