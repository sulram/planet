# 102. The bridge: three names and messages between the server and its module (decided; a fourth name asks the store, 110)

Logged 2026-10-04.

The Go server and the module it runs know each other by three names. The
module exports `reserve` and `call`, and imports `host.reply`. The server
sizes the module's inbox, writes one call in it and runs it; the module hands
back each reply before the call returns. A call and a reply are messages in
one schema, `proto/planet/module/v1`, generated for both sides. An op carries
its room whole, so a call stands alone. Every call has a deadline and the
module a ceiling of memory, and after any fault the instance is replaced and
the world started again in the new one.

**Why calls.** Marlus asked for the definitive way, the one the next plugins
stand on. A call into the module is the plain one: it runs on the actor's own
goroutine, it returns or it fails, and it costs two microseconds in a module
that does nothing else. The other way keeps `unsafe` out entirely: the module as a program that reads calls on
its standard input and writes replies on its output. It asks for a second
compilation target, a goroutine and two pipes that may each wait on the
other, and the faults worth guarding come from running a plugin's code, the
same on both ways.

**Why messages.** The names carry bytes and say nothing of what is asked.
What is asked is the schema: describe, start, an op, a session gone; a
statement, an event told. A service the next plugin asks for is one more
message, read by both sides from one file, and a payload crosses unread as
it does on the wire.

**Why the room rides whole.** An op brings who asks, the moment and everyone
in the world. The module then holds nothing of the core between two calls,
and a new instance needs the recipe alone. A line in a room of a hundred
costs 67 µs through the bridge on an Apple M5 Pro, its reach measured for
each of them. A room kept in the module is the next step when a world's size
asks for it (ROADMAP).

**The exception, and how small it is.** The workspace forbids `unsafe`, and
Rust writes that word to export a function by name and to import one. With
it forbidden, the compiler refuses exactly those: the import block and the
name of each export. Nothing here reads memory through a pointer. The server
writes a call into a buffer the module owns and sized, and the module reads
its own buffer; a reply leaves as an address and a length, which wazero
reads within the module's memory or refuses. The three declarations are
`crates/module/src/abi.rs`, each marked, in a crate that denies the rest, and
`bun run docs` holds that no other file carries the mark and every other
crate inherits the workspace's lints. The engine crosses to the browser the
same way: `wasm-bindgen` writes exported `unsafe` functions behind its macro.

**What guards a world from a plugin.** A deadline of 250 ms a call, a
ceiling of 64 MB of memory, and a limit on what a call is answered with: a
thousand replies of a megabyte each, since what the module says is kept on
the server's side, outside its ceiling. A module is refused when it is opened
unless it exports the two names as the bridge calls them. `internal/module`
attacks each with modules written by hand: one that panics, one that loops,
one that takes memory without end, one that replies without end, ones that
point outside their memory, one that speaks under another plugin's name. And
it fuzzes the real module with calls and ops of any shape, which never trap
it. After a panic the instance is no use, so a fault replaces it, and the
world is started in the new one before its next op. What a world half held in memory goes with the old
instance: for chat, how often each session spoke. What a world keeps is in
its folder, never in the module's memory.

**Where each part is.** `world::Host` is the host of world halves, in Rust
and safe, as `client` is the host of client halves: it reads a call, hands
the half its room and gathers what it said. `module` is its shell for wazero:
the three names, and how a recipe says the size of its bodies. In Go,
`internal/module` is the one package that knows the module is WASM, and it
hands each plugin over as a `world.Plugin`: the core's host of plugins, its
levels and its switches stand as 98 cut them. The built file is not
committed: `bun run module` builds it, before Go builds or tests.

**Rejected:** the module as a program on pipes; an inbox the module reads by
a pointer and frees by hand, where a wrong length reads another's memory; a
Go registry of plugins, where the module says what it carries; the room
mirrored in the module from the first day; the built module committed, where
no two machines build the same bytes.

**Lives in:** PLUGINS.md § The bridge; ARCHITECTURE.md § The server;
`proto/planet/module/v1`; `crates/world/src/host.rs`; `crates/module`;
`server/internal/module`; `scripts/module.ts`; `scripts/docs.ts`.
