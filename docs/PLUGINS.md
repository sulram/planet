# PLUGINS

How the core and a plugin are cut, what each owns and how they speak: the
shape as decided. The why is in DECISIONS 88, 91, 93, 94, 98 and 99; how much
of it is built, and in what order, is in BRIEF.md.

## The core and a plugin

- The core (88) is what a plugin stands on, and it runs with every plugin
  off. What it is made of: ARCHITECTURE.md § The core and its plugins.
- A plugin is written once, in Rust, as two halves (99): a client half the
  engine hosts and a world half the server hosts. With them, a payload on
  the wire and a panel in each front end.
- Ours, compiled in: a config at the root lists them and the build bundles
  them, so a version is the core plus the plugins chosen for it.
- Native plugins: chat, building, land, avatars, made in that order (96).
  Chat, building and avatars are extracted from where they stand today.
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
- The core owns the recipe, who a session is, its level and its stance.
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
- A volume is the unit building and land share (95): cells and their history
  are kept by volume, permission is given by volume, and a rollback restores
  a volume.

## Where a plugin lives

| Half | Where | What it holds |
|---|---|---|
| Wire | `proto/planet/<name>/v1/` | its ops and events, its own schema |
| Client | `crates/<name>/` | a `client::Plugin`: commands in, messages from the server, events out |
| World | the plugin's crate, compiled into the server's module | its ops and their levels, what each does, whom an event reaches, what is kept |
| Web | `apps/web/src/lib/plugins/<name>/` | a `WebPlugin`: its layer over the world, its label |

- `plugins.json` at the root lists a version's plugins and whether each
  starts on. `bun run plugins` writes the three registries from it
  (`crates/plugins`, `server/internal/plugins`, `$lib/plugins/index.ts`), and
  `bun run proto` generates each plugin's wire into its own halves.
- A plugin's name is a to z and `_`: it is the word before the dot.
- Chat's world half is a Go package, `server/internal/chat`, the one cut
  before the module (99, BRIEF.md).

## What the host offers

- Cut as each plugin asks for it, by extracting what exists (88). Chat cut
  these (98); building cuts a store, where an op lands, a turn in each frame,
  solids for the footing and the picture.
- **The envelope**: `Envelope { plugin, kind, payload }`, up for an op and
  down for an event. A message for a plugin that is off, or that nobody
  carries, is let pass.
- **Ops said as data**: a plugin lists its ops, each with the least level
  that may ask it. The host checks the level before the plugin sees the op.
  The answer to an op, landed or refused, is cut by building: until it is,
  an op that stops at the host is dropped unheard.
- **The statement**: the plugins that are on, each with its version, at
  `GET /api/world`, in `Welcome` and in `Plugins` when the admin switches
  one. A client mounts a plugin when the world says its name at the version
  it holds; offline, none is on.
- **The switch**: `POST /api/plugins {name, on}`, an admin's. The choice is
  kept in the world folder's `plugins.json`, which holds what the admin set
  and nothing else.
- **On the server**, services that carry no feature (99): who a session is,
  whether it may, the moment, an event told to the sessions the plugin picks.
  A store joins them with building. How near two sessions stand is the world
  half's own measure, by `topology`.
- **On the client**, a host: `send` an op up, `emit` an event over the seam,
  read a stance as a place.
- **On the web**, a layer over the world, mounted while the plugin is on, and
  a `Seam` under its own name: commands out, its events in, the pointer let
  go and taken back.
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
