# PLUGINS

How the core and a plugin are cut, what each owns and how they speak: the
shape as decided. The why is in DECISIONS 88, 91, 93, 94, 98 to 102, 104, 106,
108 and 109; how much of it is built, and in what order, is in BRIEF.md.

## The core and a plugin

- The core (88, 108) is what a world is made of and how it behaves: the
  ground, the cells, the bodies in it, their physics. It runs with every
  plugin off. What it holds today: ARCHITECTURE.md § The core and its
  plugins.
- A system is an owner in the core with its words: ops, questions, events
  (93). A plugin is what is done with systems, a tool, a rule or a panel:
  it speaks to them and holds no substance of the world.
- Two questions say which side a thing is on. With the plugin off, must
  what it made still stand and behave? Do two plugins touch it? A yes to
  either makes it the core's. A system knows no tool, key or permission.
- A plugin is written once, in Rust, as two halves (99): a client half the
  engine hosts and a world half the server hosts. With them, a payload on
  the wire and a panel in each front end.
- Ours, compiled in: a config at the root lists them and the build bundles
  them, so a version is the core plus the plugins chosen for it.
- Native plugins: chat, building, land, avatars, made in that order (96).
  Chat and building stand as plugins; avatars are extracted from where they
  stand today.
- Which plugins are on is the world's own (91). The config says whether each
  starts on; the admin switches any of them at the founding and after, in the
  world folder. A plugin switched off keeps its store untouched.
- A plugin imports the core, never another plugin, and draws through `scene`
  as the client does. `ALLOWED` in `scripts/docs.ts` holds it: a plugin's row
  names crates of the core.

## Owners

- Every piece of a world's state has one owner, the core or one plugin (93).
  The owner alone changes it; everyone else sends it an op or asks it a
  question.
- The core owns the recipe, who a session is, its level, its stance and the
  cells.
- A plugin owns what it keeps: its store in the world folder, its payloads on
  the wire, its panel.

## Three conversations

| Conversation | What it does | What may be done with it |
|---|---|---|
| Op | asks an owner to change what it holds; answers that it landed, or the code of why it was refused | checked, logged, taken back |
| Question | asks for an answer and changes nothing | asked again, kept |
| Event | says what happened, to whoever listens | let pass |

- A question is a hook when the core asks its plugins, and a reading when
  anyone asks an owner what it holds.
- A hook has a default answer. A plugin over it answers in its place, with
  the default to fall back on: the level answers who may do what and where,
  and land answers over it.
- A message is one of the three. What is two of them is two messages.
- At the seam they are `client::Command` and `client::Event`, JSON tagged by
  `type`, and a `type` with a dot is a plugin's: `chat.say`. On the wire they
  ride the envelope. A refusal is a code at both, never a sentence.

## The path of a change

1. An op arrives: from a hand's tool, a macro or an agent.
2. The host asks the permission hook with who, what and where, before the
   owner sees the op.
3. The owner applies its rule.
4. The owner keeps the result and logs the op.
5. An event tells every session.

- The rule is written once, in Rust: the client predicts with it, and the
  server decides with it, run as a WASM module through wazero (97).
- A constraint lives in the rule. One held by a panel alone is one an agent
  and a macro walk past.

## The core's words

- Two plugins meet in three words of the core, and neither names the other.
- **Who**: the account, as mundos signs it, and its level. A person or an
  agent.
- **Where**: the address. A box of it, or a plot, which is the address less
  six bits (77).
- **What**: the name of the action, the plugin's before it: `build.create`.
- A volume is the unit the cells and land share (95): cells and their history
  are kept by volume, permission is given by volume, and a rollback restores
  a volume.
- The cells are the core's (106): the volumes, their picture, the footing on
  them and the op that changes them, a gesture over a box. Building is a
  plugin of how a hand arrives at gestures: tools, kinds of construction,
  modifiers, keys, a panel. With building off, what stands, stands.

## Where a plugin lives

A plugin is one folder, `plugins/<name>/` (100):

| Part | Where in the folder | What it holds |
|---|---|---|
| Wire | `wire/planet/<name>/v1/` | its ops and events, its own schema |
| World half | `world/`, the crate `<name>-world` | a `world::Plugin`: its ops and their levels, what each does, whom an event reaches, what is kept. Also what both halves say once: name, version, kinds, wire, limits |
| Client half | `client/`, the crate `<name>-client` | a `client::Plugin`: commands in, messages from the server, events out |
| Panel | `web/`, the package `@planet/plugin-<name>` | a `WebPlugin`: its layer over the world, its label |

- The client half imports the world half, never the reverse (101). A world
  half stands on the crate `world` and never imports `client`. A panel
  imports the web front end's own `$lib`.
- `plugins.json` at the root lists a version's plugins and whether each
  starts on. `bun run plugins` writes from it the three registries
  (`crates/plugins-client` for the engine, `crates/plugins-world` for the
  server's module, `$lib/plugins/index.ts` for the web) and buf's list of
  schemas, `buf.yaml`. `bun run proto` generates each plugin's wire into its
  world half's `src/gen`.
- A new plugin with a panel adds its package to `apps/web/package.json`;
  `bun run plugins` says so when it is missing.
- A plugin's name is a to z and `_`: it is the word before the dot.
- The server holds no plugin's code and no plugin's schema: a new plugin
  touches its own folder, `plugins.json` and nothing else written by hand.

## The bridge

- The server's module is every world half of a version as one WASM file,
  `crates/module`, built by `bun run module` and embedded in the Go binary.
  `world::Host` is the host of world halves inside it.
- Four names (102, 110): the module exports `reserve` and `call`, and imports
  `host.reply` and `host.ask`. The server writes one call in the module's
  inbox and runs it; the module hands back each reply before it returns.
- A call and a reply are messages of `proto/planet/module/v1`: describe,
  start, an op, a session gone; a statement, an event told. A service a
  plugin asks for is one more message.
- An op carries its room whole: who asks, the moment, everyone here. What an
  owner keeps it asks of its store while the call runs, and what it writes
  goes back as replies, kept when the call returns, all of it or none.
- A call has a deadline of 250 ms, the module a ceiling of 64 MB, and a call
  is answered with at most 1024 replies of 1 MB each. After a fault, a trap,
  a deadline or a reply that cannot be taken, the instance is replaced, the
  op is dropped and the world starts in the new instance. What a world half
  held in memory goes with it: what a world keeps is in its folder.
- A world half answers bytes that are no call, and an op of any shape, with
  nothing: it never panics. `internal/module` holds it with hostile modules
  written by hand and two fuzz targets, `crates/module` with property tests.
- `unsafe` is written in `crates/module/src/abi.rs` alone, three times, to
  say the four names. `bun run docs` holds it.

## What the host offers

- Cut as each plugin asks for it, by extracting what exists (88): the
  envelope, the statement and the switch by chat (98); the turn, the keys,
  the pointer and the words of the cells by building (109).
- **The envelope**: `Envelope { plugin, kind, payload }`, up for an op and
  down for an event. A message for a plugin that is off, or that nobody
  carries, is let pass.
- **Ops said as data**: a plugin lists its ops, each with the least level
  that may ask it. The host checks the level before the plugin sees the op.
  An op asked with an id is answered: it landed, or the code of why not.
- **The statement**: the plugins that are on, each with its version, at
  `GET /api/world`, in `Welcome` and in `Plugins` when the admin switches
  one. A client mounts a plugin when the world says its name at the version
  it holds; with no world, none is on (104).
- **The switch**: `POST /api/plugins {name, on}`, an admin's. The choice is
  kept in the world folder's `plugins.json`, which holds what the admin set
  and nothing else.
- **On the server**, services that carry no feature (99), a `world::Room`:
  the moment, who is here, the body's measure, an event told to the sessions
  the plugin picks. Whether a session may is asked before the plugin sees the
  op, and a store of its own keeps what it keeps. How near two sessions stand
  is the world half's own sum, with the room's measure.
- **On the client**, a host: `send` an op up, `emit` an event over the seam,
  read a stance as a place.
- **A turn**: each frame, after the body moved and before the picture, with
  the eye, the pointer and those of the plugin's keys that are held. A plugin
  says when it owes work (`busy`), does it at once for a picture (`settle`),
  and puts down what it has in hand when its world switches it off (`rest`).
- **Keys by name**: a plugin lists its keys as data, each a name, a key of a
  keyboard as the web names it, a chord, and held or heard once. A shell
  hands over the keys the client says are asked for.
- **The pointer**: one plugin has it at a time. With it the pointer is free
  to aim and its button is the plugin's.
- **The cells' words**: read freely, `holds`, `cell`, `corner`, `sight` from
  a point along a direction, `history`; asked as ops, `open`, `apply`
  (gestures as one change), `preview`, `take_back`, `put_back`. Where cells
  stand is a `Seat`, a sector today, never a bare address.
- **The body's words**: where its feet are, a floor to be lifted onto and,
  for the plugin with the pointer, flight through cells.
- **On the web**, a layer over the world, mounted while the plugin is on, and
  a `Seam` under its own name: commands out, its events in, the pointer let
  go and taken back. The core's readings come as props, the level and the
  palette among them, and a plugin's keys are listed in the help by it.
- A hook is written when a plugin asks: who may do what and where, which
  avatars are offered.

## A person and an agent

- What a world offers, it offers to a person and to an agent alike (94).
- A capability is a command at the seam, named, its parameters said. A tool
  is how a hand composes one; an agent sends the command itself.
- Every op a tool makes has a command that asks for it by its parameters.
- Whatever a picture shows has a reading that says the same as data: the
  ground, the cells in a box, who is near, where an account may build.
- An agent is an account as a person is: a door, a level, permissions.
