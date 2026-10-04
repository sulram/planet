# Plugins: where the host's seams lead

Drawn 2026-10-04, with chat and building standing as plugins and the cells
being kept. Nothing here is decided or built. It is the whole that the next
cuts of the host are measured against, so that each one fits a drawing and is
more than the sum of what two plugins happened to ask.

## The thread: a plugin says itself once

A plugin already says part of itself as data: its ops and their levels
(`world::Plugin::ops`), its keys (`client::Plugin::keys`). The rest is said
twice. Building's panel repeats in TypeScript the tools, the bases, the
platform sizes, the reasons for a refusal, the names of every command and
event, the plugin's name and version, and its keys again as hints
(`plugins/build/web/protocol.ts`, `index.ts`).

The drawing: a plugin's description is complete. Every command at the seam
with its parameters (a name, a type, a range or a list of choices, a
default), every event with its shape, every refusal code, every key, every
limit. It is derived from the types the plugin parses with, so it could not
say something the plugin does not do.

One description would then have five readers:

1. **The panel's types.** `bun run plugins` writes the `.ts` a panel imports.
   A panel that names a tool the crate dropped stops compiling.
2. **A panel drawn from data.** A command whose parameters are a choice, a
   colour of the palette and a size is a row of buttons, a swatch and a
   slider on any screen. A plugin with no panel written would still have
   one, in egui on a desktop and in a headset. A Svelte panel becomes the
   bespoke one a plugin writes when the drawn one is not enough. This would
   answer OPEN's question on a panel outside Svelte.
3. **An agent's tools.** A command with a name, a sentence and a JSON Schema
   of its parameters is a tool of the Model Context Protocol, as it stands.
   The seam is JSON already.
4. **The help and the docs.** The keys in the help, a page a plugin, the
   line in mundos's catalog that says what a world offers.
5. **A check.** A command arriving with a parameter out of its range is
   refused by the host before the plugin sees it, as a level is today.

Three ways of writing it, side by side:

- **Derived from the Rust types**, by a derive that yields JSON Schema, with
  attributes for ranges and choices. The description follows the parser by
  construction. It could run in a native binary at `bun run plugins` alone,
  so the engine's WASM would carry no byte of it. My lean.
- **A table by hand**, as `ops()` and `keys()` are: no dependency, and a
  test holds it to the parser.
- **The seam in protobuf**, as the wire is: one schema language end to end
  and TypeScript from buf. It would cost the seam its readable JSON, and an
  agent's tools want JSON Schema whatever the seam speaks.

The world would serve the description beside the statement, so a client and
an agent read it from the world they entered.

## The agent is the second front end

A seam is real with two implementations (65), and the command and event seam
has one family of them: shells with a picture. BRIEF's last step asks for a
second way of building to make building's seams real. An agent that builds
by parameters is a second way of building, and it would make three things
real at once: every op a tool makes asked for by its parameters, every
picture with a reading that says the same as data, and the description
above with a reader that is not ours.

Its shape could be a shell: `client` with no renderer, speaking the Model
Context Protocol on one side and the world's socket on the other. Its tools
would be the commands of the plugins the world has on, at the level of the
account it entered with. Its readings: the ground under a box, the cells in
a box, who is near, where it may build. A picture from a pose would come from
the headless renderer that `bun run shot` already drives.

Two doors, side by side: an agent that enters with a person's own key and
builds as that person, which needs nothing new from mundos; and an agent
with a door of its own, an account as a person is (94), which is mundos's
work. The first would be enough to see an agent lay a platform in a world
while a person watches, and that picture is what the project says it is for.

## One bridge, both halves

A world half lives behind three names and messages (102). A client half is
linked into the engine and handed a `Host`. Most of that host is plain data
in and out: `send`, `emit`, `sight`, `apply`. Two words lend a structure of
the core instead: `cells()` and `sphere()`.

If every word of the client's host could be said as a message, a client half
could live behind the same three names as a world half. A plugin would then
be two WASM files and a description, and a world could load one without a
new build of the engine: on the server through wazero, as today, and in the
browser as a second module beside the engine. DECISIONS 88 set loading at
run time aside because a module in a browser links nothing after it is
built. Messages need no linking.

None of this would be built before someone outside writes a plugin. What it
would ask today is discipline in one place: a new word of the host takes and
gives plain data. The frame is the referee, and a turn across a boundary is
measured in WASM before it is believed.

The near step for a plugin written by a stranger is source: `plugins.json`
naming a plugin by where it is fetched from and at which version, and
`bun run plugins` fetching it. A version of a world is an image, so a
stranger's plugin already reaches a world through a build.

## What moves, drawn with avatars

The wire keys what moves by session: a stance for each session, a figure for
each session. A vehicle parked with nobody in it, a thing put down, a ship
with two riders: none is a session. They would be things with an id of their
own, a stance, and parts: a figure, a seat, a thrust, a hold. A session
would drive one of them.

The step that extracts avatars is where the core learns to draw a figure for
something that moves. If that something were keyed by its own id from that
step, a vehicle would be a second kind of it and a wire kept as it is. Keyed
by session, the first vehicle would reopen the wire, presence, and every
plugin that reads who is where. The library, `hecs` or another, could still
wait for the second kind (OPEN). The id is what is dear to undo.

Physics the server decides would be a simulation in the module: state held
between calls, and a turn. The cells, once kept, would be the first system
to hold state there and to read a store through the bridge, and a turn would
be one more call when something that moves asks for it. A store a system
asks for by key would serve a simulation too.

## An order, if a campaign pulled it

1. The cells kept (BRIEF 10), as it stands.
2. The description, read first by the panel's types: the smallest reader,
   and it removes what is said twice.
3. The agent's shell, with building's commands by parameters. It would be
   BRIEF 13 arriving early, and it would give step 5's upgrade something
   worth shipping.
4. Land, tried by a person and an agent from its first day.
5. Avatars, with an id for what moves.

## What would prove it wrong

- A derive whose schema is too poor to draw a panel from: ranges, choices
  and colours not sayable as attributes. Then the table by hand.
- A turn across a module boundary that costs the frame more than it has.
  Then loading stays on the server's side alone.
- An agent that needs to see more than readings say. Then the picture from
  a pose is its first tool, ahead of building.
