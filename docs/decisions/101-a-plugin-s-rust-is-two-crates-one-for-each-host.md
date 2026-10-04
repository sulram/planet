# 101. A plugin's Rust is two crates, one for each host: `client/` and `world/` (decided)

Logged 2026-10-04.

A plugin's folder holds two crates: `world/`, its world half, and `client/`,
its client half. The world half owns what both say once: the plugin's name
and version, its op and event kinds, its wire and its limits. The client half
imports it, and never the other way. A world half stands on the crate
`world`: the plugin as the server hosts it, the room it is handed and the
body's measure. It never imports `client`.

**Why two.** There are two hosts. The engine links the client half, with
`client` under it: the controller, the scene, the generator, everything a
tool touches. The server's module links the world half and has no use for
any of that. One crate for both asks either for `client` inside the server's
module, or for every word of `client` a plugin uses to be said again as a
contract in a crate between them, with one implementation each (65).
Marlus's word for it: as Roblox has it, a script for the client and a script
for the server.

**Why the arrow points at the world half.** A client predicts with the rule
the server decides with (97), so the client half reads the world half and the
reverse never holds. For chat that is its wire and its names; for building it
is the rule itself.

**Why a crate named `world`.** It is to a world half what `client` is to a
client half, and it says in Rust what `internal/world` says in `plugin.go`:
`Plugin`, `Room`, `Who`, `Op`. `Measure` is how far apart two stances stand,
by `topology` and the recipe's own size, where `near.go` holds a mirror fixed
at `2^16`. The seam has two implementations from its first day: the module's
room, and the room a test hands a world half.

**What was tried.** Cargo takes `plugins/*/client` and `plugins/*/world` as
members and leaves the rest of a folder alone, a plugin with a world half
only among them. Chat's tests in Go are its world half's in Rust, with one
more: someone who has not said where they are, or stands on another body, is
not near.

**Rejected:** one crate over a contract crate both hosts implement; one crate
that imports `client`, linked into the module; the world half nested under a
client crate at the folder's root, a crate inside a crate.

**Lives in:** PLUGINS.md § Where a plugin lives; CLAUDE.md § Layout;
`crates/world`; `plugins/chat/world`; `plugins/chat/client`;
`scripts/docs.ts` (`ALLOWED`).
