# 99. A plugin has no Go: its world half is Rust, and the server hosts it (decided; the grid of cells is the core's, 106)

Logged 2026-10-04.

A plugin is written once, in Rust, as two halves: a client half the engine
hosts and a world half the server hosts. The world half is all of a plugin
that runs where the world is decided: the ops it offers and the level each
asks, what an op does, whom an event reaches, what is kept. It is compiled
into the one WASM module the Go server runs through wazero (97), beside the
body's measures and the generator. The server holds no line of any plugin.
It offers services that carry no feature: who a session is, whether it may,
the moment, an event told to the sessions picked, a store. A service is
written when a plugin asks for it (88).

**Why the whole half, and not the rule alone.** 97 put what an op does to a
kept thing in Rust and left carrying to Go, and 98 read chat as carrying: its
limits, its reach and its relay are a Go package, and how far apart two
stances stand is mirrored from `topology` into `near.go`, where it measures
every world at the reference size. The first plugin sets the pattern for the
next, and that pattern is one feature in three languages, with a server half
split between Go and the module from building on. What a plugin decides on
the server is its own whether or not a kept thing changes: a line too long, a
line said too often, who stands near enough to hear. With the whole half in
the module, a new plugin is Rust and a panel and never touches the server;
the measure of the body is `topology`'s on both sides; and a limit is one
constant both halves read.

**Why the server still decides.** Marlus asked for a server dumb enough that
a plugin only uses its infrastructure. It is dumb about features, and it is
still where a plugin decides: the world half applies an op inside the server,
at the server's moment, with the server's word on who asks. Land, a wallet
and everything kept stand on that. A relay that orders and carries what
clients decided, a Nostr relay or a Croquet reflector, leaves the content to
the client, and the client is what someone without permission edits.

**Why services, and few.** A service is what any plugin may ask and none
owns. Messaging is an event told to the sessions within a reach, and chat is
the first to ask it. A grid of cells is building's rule and stays building's:
a service that knows a feature is that plugin's server half under another
name, written in Go beside its Rust (97), and it puts into the core what 88
keeps out of it. The services to come are named by who asks: a store and an
answer to an op by building, the permission hook by building and land, a
question asked outside the world by a wallet, more than one body for a
session by the first vehicle.

**Why Go stays.** A plugin is written against a host in Rust on both sides,
so the server's language reaches no plugin. What is left to the server is the
door, the room, the world folder and the box the module runs in, which Go
does well (02), and changing it later costs the server's two thousand lines
and nothing of a plugin. The question returns the day the core in Go mirrors
the engine's measures again.

**What it asks.** The module of 97 is cut by chat, ahead of building: the
bridge between Go and the module, written by hand over wazero as bytes in and
bytes out; the world half's contract in Rust, the mirror of `client::Plugin`
and its `Host`; and chat's world half in place of `internal/chat` and
`near.go`. Until it is built, chat's server half stands in Go as 98 cut it.
The world half cannot import `client`, so how the two halves share a plugin's
crate is settled with that work (OPEN).

**Rejected:** chat's limits kept in Go (98), a server half in two languages;
a server in Rust, which answers what the host is written in before where a
plugin's logic lives, and is a rewrite no plugin gains from; a relay that
applies nothing (97); protocols of features in the core, messaging, a grid of
cells, bodies that move, each drawn ahead of the plugin that asks (65) and
each a rule written twice; a plugin that is a panel alone speaking to a
service, which a headset, a desktop and an agent each write again (94).

**Lives in:** PLUGINS.md; ARCHITECTURE.md § The server; BRIEF.md;
ROADMAP.md § Plugins: native; GLOSSARY.md; OPEN.md.
