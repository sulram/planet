# ARCHITECTURE

The shape as decided. The why lives in DECISIONS.md, open points in OPEN.md,
and how much of it is built in BRIEF.md. A line marked (p) is proposed and
not yet confirmed.

## A world, hosted

- An instance is one world (87): one container from a published version, one
  world folder that outlives it, one bucket folder, one port. mundos
  (`~/Dev/mundos`) creates, stops, addresses, lists and upgrades it. How it
  is hosted: DEPLOY.md.
- A world is born unfounded: its recipe is yet to be chosen. The first admin
  to enter previews a planet offline, turns its knobs and founds the world
  from what is on the screen; the recipe is frozen from then on. Until then
  everyone else is told the world is on its way.
- A world answers for itself at `GET /api/world`: what it says about itself,
  for mundos's catalog and door, and what it speaks (91): the engine's
  version, the wire's, and the plugins that are on.
- An update is a copy: mundos copies the folder and the bucket folder into
  the next generation on the new version, and a promotion moves the address.

## Identity and levels

- Who is who is mundos's: accounts live there, and the world checks a
  signature and keeps a level.
- Every load passes through mundos's door: a page with no `#identity=` goes to
  `<mundos>/enter?host=<host>`, back with a token or `guest`, and `?lang=` the
  language read there, kept as the page's. The pose is kept across the hop.
- The token is a JWT signed with Ed25519 (`EdDSA`): `iss` `mundos`, `aud` the
  world's name, `sub` the account's id, `name`, `level`, `exp` a minute on,
  read with 30 s of leeway. The server checks it with `MUNDOS_PUBLIC_KEY`.
- The page hands the token to the world once, on entering, and gets a key:
  kept in the page's memory, shown on the socket's URL and on a founding, held
  by the server in memory for half a day. The level holds for the life of the
  page; a role changed in mundos holds on the next load.
- A link that drops opens again with the same key. A key stands while the
  server remembers it; past a restart or its half day, the page goes through
  the door for a new one.

| Level | Who | May |
|---|---|---|
| `admin` | a superadmin of mundos, or an admin of this world | found the world, choose its plugins, build |
| `builder` | a builder of this world | build |
| `signed_in` | an account with no role here | walk, under the account's name |
| `anonymous` | a guest | walk, under a name of their own choosing |

- `Welcome` says the session's level, and the client offers its tools by it.
  A stroke that is an op is checked by the actor against the same level,
  through the permission hook.
- Alone, where `MUNDOS_PUBLIC_KEY` is unset, every session has
  `PLANET_DEV_LEVEL`: `anonymous` by default, `admin` under `bun run dev`.

## The core and its plugins

- The core (88, 108) is what a world is made of and how it behaves, as
  systems: the body (`topology`, `worldgen`, the recipe), the picture
  (`render`, `scene`), a body walking it (controller, footing, camera), the
  cells (`voxel`, 106), the link (socket, session, level, a stance for each
  session) and the host of plugins. It runs with every plugin off.
- What is done with it is a plugin (99): a tool, a rule, a panel, as two
  halves in Rust and a wire, compiled in and on or off for a world.
- Every state has one owner, a system of the core or one plugin, and what
  crosses a seam is an op, a question or an event (93). How a plugin is cut,
  what it owns and how it speaks: PLUGINS.md.

## The server

- One Go executable is the instance (90): the world socket, the world's
  routes, the files when there is no bucket, and the web front end as static
  files. `cmd/planet` is the entry, `internal/world` the core, `internal/api`
  the routes.
- A plugin's world half is its Rust, run in one WASM module through wazero
  (97, 99): `internal/module` embeds it and hands each plugin to the core. Go
  offers services, a store among them, and reads no payload. The bridge is
  four names and one schema, with a deadline and a ceiling of memory (102, 110).
- Wire: protobuf, one message per binary WebSocket frame, `proto/` the single
  source (66). `Hello` says the protocol version; any other is refused.
  `Welcome` carries the session, its level, the recipe, so a client checks it
  stands in the world the server holds, and the plugins that are on. A
  plugin's message rides an `Envelope`: its name, a kind and its own payload.
- The hub holds the world's actor: started on the first session, gone after
  the last. The actor keeps every session's last stance and relays what
  changed at 15 Hz, in one frame encoded once, with a heartbeat every two
  seconds so a silent link is a dead one on both sides. A client too slow to
  take its frames is dropped; the actor never waits for a client.
- Routes: `GET /api/world` says what the world is, `POST /api/enter` trades
  the door's token for a key, `GET /api/me` says whether a key stands,
  `POST /api/world` is the founding, `POST /api/plugins` switches a plugin,
  `GET /api/socket` is the world socket. A route that changes the world takes
  the key and checks its level. An error is a code, never a sentence.
- The actor is the host of plugins on the server (PLUGINS.md): it finds the
  plugin an envelope names, checks the level its op asks and hands it over.
- Chat (69) is a plugin, and the server holds none of it: `say` up, `said`
  down, relayed to everyone in scope, the speaker included, never stored.
  `near` reaches `NEAR_BLOCKS` on the same body, measured by its world half
  with the recipe's own size; `world` reaches every body. A line said with
  `here` comes back with the speaker's stance as the actor holds it. Its
  limits are its own: `LINE_CHARS`, five lines in five seconds.
- The world folder (89): `world.json`, the recipe the founding froze;
  `plugins.json`, the plugins its admin switched; and one SQLite file for
  each plugin that keeps things, moved forward by that plugin when it starts.
  Permission decides, the plugin's op log records.

## Files

- A heavy file, an image, a video, a GLB, is named by the hash of its content
  and never changes (89). A record names it `asset://<hash>.<ext>`.
- A session that may build sends it to the world server, which names it and
  writes it to the bucket folder mundos named for this generation. Browsers
  read it from the address in front of the bucket. Alone, the world folder
  holds the files and the server serves them. On start a world deletes
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
- A front end sends commands and renders events (90). The web one is static
  files, so what any front end does goes through the seam and the world's
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
- Building is a plugin (106, 109): `build.take` takes a tool or `null`, then
  `build.paint`, `.finish`, `.edge`, `.platform`, `.lay` with its base,
  `.open`, `.close`, `.undo`, `.redo`; it says `build.hand`, `.over`, `.refused`,
  `.history`, and the core says `palette`. With a tool in hand the plugin has
  the pointer: it is free, the primary button is the tool's, the secondary
  one looks. Each shell binds the keys it asks for by name: B, 1 2 3 4 take a
  tool, Alt turns a stroke, Escape drops the stroke and then the tool, Cmd or
  Ctrl Z takes a change back and with Shift puts it back.
- Desktop: `ui-native`, where all of egui lives. egui and egui-winit from
  crates, the painter ours (`egui-wgpu` pins an older wgpu). The shell hands it
  window events first while the pointer is free; `shot --panel` paints it over
  the headless picture.
- The link is the shell's, the protocol the client's: `shell-web` opens the
  socket (`Engine.connect(url)`), hands every frame to `Client::receive` and
  sends what `drain_outbound` queues. The client says hello, keeps the peers,
  sends its own stance when it changed and as a heartbeat, and reports
  `session`, `peers`, `statement` and `anchors` over the seam, and `settled`
  when the streamer has nothing left to build for the view. `anchors` is
  where every head in view is on the screen, each frame, so nametags and
  balloons are a front end's DOM and never a render feature. A shell plugs
  in the version's plugins (`plugins::all`), and a command or an event whose
  `type` has a dot is a plugin's: `chat.say`. The desktop shell has no socket
  yet (ROADMAP § Other screens).
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
  is the source, `pt.ts` and `zh.ts` match it. A plugin's words are in its own
  folder, read through `words`.
- The renderer accepts N views (1 desktop, 2 XR) and writes reversed depth.

## Future shapes already accounted for

- Another screen is a shell around the core, a door in mundos for a device
  (a code on a screen), and each plugin's panel in that front end (OPEN.md).
- Scale bands (giant, human, bug): the octree descends into sub-grids; the
  server streams only the levels your scale may see. Secrets are enforced by
  the server, not by the client.
- Destruction: the instigator predicts locally and sends an op; the server
  validates and broadcasts the event with a seed; debris is local and cosmetic.
