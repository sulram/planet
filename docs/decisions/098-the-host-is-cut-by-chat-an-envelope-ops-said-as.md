# 98. The host is cut by chat: an envelope, ops said as data, a statement and one config (decided; a plugin's server half is 99's)

Logged 2026-10-04.

The wire carries a plugin's message in an envelope of three fields: the
plugin's name, a kind and a payload the core never reads. A plugin says its
ops as data, each with the least level that may ask it, and the host checks
that before the plugin sees the op. A world says which plugins are on in its
welcome, at `GET /api/world` and again whenever its admin switches one, and
the admin's word is a file of its own in the world folder. A version's
plugins are listed once, in `plugins.json`, and `bun run plugins` writes each
toolchain's registry from it. Chat is the first plugin: its wire, its crate,
its package and its layer left the core, and a world runs with it on or off.

**Why a kind beside the name.** The host asks who may do what before the
owner applies it (93), and it reads no payload. With the kind in the
envelope the host knows what is asked: it finds the op the plugin said, and
the level it asks. The same list is what a front end offers its tools by and
what an agent reads to know what it may ask (94).

**Why the level, for now.** The permission hook's default answer is the level
an op asks, and that is all chat needs: anyone speaks. Where an op lands joins
the question when building asks it, and a plugin that answers over the level
when land does. A mute is one more answer over it.

**Why an op that stops is dropped unheard.** It is chat's own rule for a line
too long or too fast (69). An answer to an op, landed or refused with a code,
needs something to match it to, and building's stroke is the first op that
has it: the answer is cut there.

**Why the switches are a file of their own.** `plugins.json` in the world
folder holds what the admin set and nothing else, so a plugin a later version
adds starts as that version's config says, and the recipe's file stays
written once. An upgrade copies it with the folder.

**Why one config and three registries.** A crate, a Go package and a list of
Svelte components are how each toolchain composes at build, and none of them
reads another's. The config is the one place a person edits; the three are
generated and committed, so an image builds with no generator, and
`bun run check` fails when they drift.

**Why chat's limits stay in Go.** A line changes nothing a plugin owns, so
there is no rule to run as WASM (97): relaying, counting a rate and measuring
who is near are carrying, and carrying is the server's.

**What a plugin is given, as far as chat asked.** On the server, a room: the
moment, who is near whom, and a way to tell an event to the sessions it
picks. On the client, a host: a way to send its op up, to say an event over
the seam and to read a stance as a place. Over the seam, a `type` with a dot
in it is a plugin's: `chat.say`, `chat.said`. On the web, a layer over the
world and the seam under its own name; the nametag over a head stays the
core's and the balloon above it is chat's.

**What chat did not cut.** A store, where an op lands, a turn in the frame,
solids for the footing and the picture: building asks for each (96).

**Rejected:** an envelope with the name alone, where the host cannot know
what is asked; the kind as a number, which reads in no log and in no agent's
list; the switches inside `world.json`; the statement in the welcome alone,
which a client that has yet to enter cannot ask for; registries written by
hand in three places; chat left in the stage behind a flag.

**Lives in:** PLUGINS.md; ARCHITECTURE.md § The server; `proto/planet/v1`
and `proto/planet/chat/v1`; `server/internal/world/plugin.go`;
`crates/client/src/plugin.rs`; `apps/web/src/lib/plugins`; `plugins.json`.
